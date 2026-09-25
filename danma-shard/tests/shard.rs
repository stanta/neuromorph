use danma_core::{Activation, Config, Error as CoreError, Feedback, FeedbackSource, FeedbackStatus, Forward, Neuron, SynapticInput};
use danma_shard::{Shard, ShardError};

fn config() -> Config {
    Config {
        activation: Activation::Linear,
        learning_rate: 0.1,
        activation_ttl_ms: 2_000,
        replay_retention_ms: 2_000,
        max_live_events: 16,
        max_staleness_versions: 8,
    }
}

fn neuron(id: u64, incoming: u64, weight: f32) -> Neuron {
    Neuron::new(id, 0.0, [(incoming, weight)], config()).unwrap()
}

fn forward(id: u128, source: u64, source_event: u128, value: f32, expected: Vec<FeedbackSource>) -> Forward {
    Forward {
        event_id: id,
        trace_id: 77,
        now_ms: 1_000,
        inputs: vec![SynapticInput { from: source, source_event_id: source_event, value }],
        expected,
    }
}

#[tokio::test]
async fn two_workers_host_four_independently_addressable_neurons() {
    let shard = Shard::new(vec![
        neuron(1, 99, 2.0),
        neuron(2, 1, 3.0),
        neuron(3, 2, 4.0),
        neuron(4, 3, 5.0),
    ], 2, 8).unwrap();
    assert_eq!(shard.worker_count(), 2);
    assert_eq!(shard.neuron_ids(), vec![1, 2, 3, 4]);
    assert!(shard.contains(4));
    assert!(!shard.contains(99));

    let out = shard.forward(1, forward(10, 99, 1, 1.0, vec![
        FeedbackSource::Neuron { neuron_id: 2, event_id: 20 },
    ])).await.unwrap();
    assert_eq!(out, 2.0);
    assert_eq!(shard.forward(2, forward(20, 1, 10, out, vec![
        FeedbackSource::Teacher,
    ])).await.unwrap(), 6.0);
    let trace = shard.trace(1, 10).await.unwrap().unwrap();
    assert_eq!(trace.trace_id, 77);
    assert_eq!(trace.expected_contributions, 1);
    assert_eq!(shard.inspect(1).await.unwrap().weights.get(&99), Some(&2.0));
    assert_eq!(shard.inspect(4).await.unwrap().version, 0);
}

#[tokio::test]
async fn feedback_routes_between_workers_without_retraining_an_event() {
    let shard = Shard::new(vec![neuron(1, 99, 2.0), neuron(2, 1, 3.0)], 2, 4).unwrap();
    let source = FeedbackSource::Neuron { neuron_id: 2, event_id: 20 };
    shard.forward(1, forward(10, 99, 1, 1.0, vec![source])).await.unwrap();
    shard.forward(2, forward(20, 1, 10, 2.0, vec![FeedbackSource::Teacher])).await.unwrap();

    let end = shard.backward(2, Feedback {
        event_id: 20, from: FeedbackSource::Teacher, gradient: 6.0,
        expires_at_ms: 1_500, hops_left: 4,
    }, 1_001).await.unwrap();
    let FeedbackStatus::Applied { upstream, .. } = end else { panic!("downstream must learn"); };
    assert_eq!(upstream.len(), 1);
    assert_eq!(upstream[0].target_neuron_id, 1);
    assert!(matches!(
        shard.backward(1, upstream[0].feedback.clone(), 1_002).await.unwrap(),
        FeedbackStatus::Applied { .. }
    ));
    assert_eq!(shard.backward(1, upstream[0].feedback.clone(), 1_003).await.unwrap(), FeedbackStatus::IgnoredDuplicate);
    assert_eq!(shard.inspect(1).await.unwrap().version, 1);
    assert_eq!(shard.inspect(2).await.unwrap().version, 1);
    assert!(shard.trace(1, 10).await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_layout_and_unknown_neuron_fail_before_side_effect() {
    assert!(matches!(Shard::new(vec![], 2, 8), Err(ShardError::InvalidConfiguration)));
    assert!(matches!(
        Shard::new(vec![neuron(1, 99, 2.0)], 0, 8),
        Err(ShardError::InvalidConfiguration)
    ));
    assert!(matches!(
        Shard::new(vec![neuron(1, 99, 2.0)], 1, 0),
        Err(ShardError::InvalidConfiguration)
    ));
    assert!(matches!(
        Shard::new(vec![neuron(1, 99, 2.0), neuron(1, 99, 3.0)], 2, 8),
        Err(ShardError::DuplicateNeuron(1))
    ));
    let shard = Shard::new(vec![neuron(1, 99, 2.0)], 2, 8).unwrap();
    assert_eq!(shard.worker_count(), 1);
    assert!(matches!(shard.inspect(999).await, Err(ShardError::UnknownNeuron(999))));
    assert!(matches!(
        shard.backward(1, Feedback {
            event_id: 999, from: FeedbackSource::Teacher, gradient: 1.0,
            expires_at_ms: 2_000, hops_left: 2,
        }, 1_001).await,
        Err(ShardError::Core(CoreError::UnknownEvent))
    ));
    assert_eq!(shard.inspect(1).await.unwrap().version, 0);
}
