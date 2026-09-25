//! Three independent neuron owners exchange forward outputs and routed gradients.
//! This is an in-process protocol test, NOT a TCP cluster or production gossip.
use danma_core::{
    Activation, Config, Feedback, FeedbackSource, FeedbackStatus, Forward, Neuron, SynapticInput,
};

fn config() -> Config {
    Config {
        activation: Activation::Linear,
        learning_rate: 0.1,
        activation_ttl_ms: 500,
        replay_retention_ms: 500,
        max_live_events: 16,
        max_staleness_versions: 8,
    }
}

#[test]
fn a_gradient_returns_across_three_independently_owned_neurons() {
    let mut a = Neuron::new(1, 0.0, [(99, 2.0)], config()).unwrap();
    let mut b = Neuron::new(2, 0.0, [(1, 3.0)], config()).unwrap();
    let mut c = Neuron::new(3, 0.0, [(2, 4.0)], config()).unwrap();

    let a_value = a.forward(Forward {
        event_id: 10, trace_id: 1, now_ms: 1_000,
        inputs: vec![SynapticInput { from: 99, source_event_id: 1, value: 1.0 }],
        expected: vec![FeedbackSource::Neuron { neuron_id: 2, event_id: 20 }],
    }).unwrap();
    let b_value = b.forward(Forward {
        event_id: 20, trace_id: 1, now_ms: 1_001,
        inputs: vec![SynapticInput { from: 1, source_event_id: 10, value: a_value }],
        expected: vec![FeedbackSource::Neuron { neuron_id: 3, event_id: 30 }],
    }).unwrap();
    let c_value = c.forward(Forward {
        event_id: 30, trace_id: 1, now_ms: 1_002,
        inputs: vec![SynapticInput { from: 2, source_event_id: 20, value: b_value }],
        expected: vec![FeedbackSource::Teacher],
    }).unwrap();
    assert_eq!(c_value, 24.0);

    let FeedbackStatus::Applied { upstream: c_to_b, .. } = c.backward(Feedback {
        event_id: 30, from: FeedbackSource::Teacher, gradient: c_value,
        expires_at_ms: 1_500, hops_left: 5,
    }, 1_003).unwrap() else { panic!("C did not learn") };
    assert_eq!(c_to_b.len(), 1);
    assert_eq!(c_to_b[0].target_neuron_id, 2);

    let FeedbackStatus::Applied { upstream: b_to_a, .. } =
        b.backward(c_to_b[0].feedback.clone(), 1_004).unwrap() else { panic!("B did not learn") };
    assert_eq!(b_to_a[0].target_neuron_id, 1);
    assert_eq!(b_to_a[0].feedback.event_id, 10);
    assert!(matches!(
        a.backward(b_to_a[0].feedback.clone(), 1_005).unwrap(),
        FeedbackStatus::Applied { .. }
    ));
    assert_eq!(a.version(), 1);
    assert_eq!(b.version(), 1);
    assert_eq!(c.version(), 1);
    assert_eq!(a.backward(b_to_a[0].feedback.clone(), 1_006).unwrap(), FeedbackStatus::IgnoredDuplicate);
}
