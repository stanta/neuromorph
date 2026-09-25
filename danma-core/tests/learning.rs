use danma_core::{
    Activation, Config, Error, Feedback, FeedbackSource, FeedbackStatus, Forward, Neuron,
    SynapticInput,
};

fn config() -> Config {
    Config {
        activation: Activation::Linear,
        learning_rate: 0.1,
        activation_ttl_ms: 100,
        replay_retention_ms: 200,
        max_live_events: 16,
        max_staleness_versions: 8,
    }
}

fn forward(event_id: u128, expected: Vec<FeedbackSource>) -> Forward {
    Forward {
        event_id,
        trace_id: 42,
        now_ms: 1_000,
        inputs: vec![SynapticInput {
            from: 1,
            source_event_id: 10,
            value: 3.0,
        }],
        expected,
    }
}

fn feedback(event_id: u128, from: FeedbackSource, gradient: f32) -> Feedback {
    Feedback {
        event_id,
        from,
        gradient,
        expires_at_ms: 1_100,
        hops_left: 4,
    }
}

#[test]
fn forward_uses_actual_weights_and_retains_a_trace() {
    let mut n = Neuron::new(2, 1.0, [(1, 2.0)], config()).unwrap();
    assert_eq!(n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap(), 7.0);
    let trace = n.trace(20).unwrap();
    assert_eq!(trace.trace_id, 42);
    assert_eq!(trace.output, 7.0);
    assert_eq!(trace.parameter_version, 0);
}

#[test]
fn distinct_feedback_branches_accumulate_once_and_use_forward_weights() {
    let mut n = Neuron::new(2, 1.0, [(1, 2.0)], config()).unwrap();
    let c = FeedbackSource::Neuron { neuron_id: 3, event_id: 30 };
    let d = FeedbackSource::Neuron { neuron_id: 4, event_id: 40 };
    n.forward(forward(20, vec![c, d])).unwrap();

    assert!(matches!(
        n.backward(feedback(20, c, 0.5), 1_001).unwrap(),
        FeedbackStatus::Pending { remaining: 1 }
    ));
    assert_eq!(n.backward(feedback(20, c, 999.0), 1_002).unwrap(), FeedbackStatus::IgnoredDuplicate);
    assert_eq!(n.weight(1), Some(2.0));

    let result = n.backward(feedback(20, d, 1.5), 1_003).unwrap();
    let FeedbackStatus::Applied { upstream, version } = result else {
        panic!("expected a single completed update");
    };
    assert_eq!(version, 1);
    assert!((n.weight(1).unwrap() - 1.4).abs() < 1e-6);
    assert!((n.bias() - 0.8).abs() < 1e-6);
    assert_eq!(upstream.len(), 1);
    assert_eq!(upstream[0].target_neuron_id, 1);
    assert_eq!(upstream[0].feedback.event_id, 10);
    assert_eq!(upstream[0].feedback.gradient, 4.0);
    assert_eq!(upstream[0].feedback.hops_left, 3);
    assert_eq!(n.backward(feedback(20, d, 1.5), 1_004).unwrap(), FeedbackStatus::IgnoredDuplicate);
    assert_eq!(n.weight(1), Some(1.4));
}

#[test]
fn feedback_after_activation_ttl_cannot_modify_weights() {
    let mut n = Neuron::new(2, 0.0, [(1, 2.0)], config()).unwrap();
    n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap();
    assert_eq!(
        n.backward(feedback(20, FeedbackSource::Teacher, 10.0), 1_100).unwrap(),
        FeedbackStatus::Expired
    );
    assert_eq!(n.weight(1), Some(2.0));
    assert_eq!(n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap_err(), Error::DuplicateEvent);
}

#[test]
fn expired_packet_is_rejected_without_closing_a_live_activation() {
    let mut n = Neuron::new(2, 0.0, [(1, 2.0)], config()).unwrap();
    n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap();
    let mut packet = feedback(20, FeedbackSource::Teacher, 1.0);
    packet.expires_at_ms = 1_001;
    assert_eq!(n.backward(packet, 1_002).unwrap(), FeedbackStatus::Expired);
    assert!(matches!(
        n.backward(feedback(20, FeedbackSource::Teacher, 1.0), 1_003).unwrap(),
        FeedbackStatus::Applied { .. }
    ));
}

#[test]
fn stale_activation_does_not_replay_a_gradient() {
    let mut cfg = config();
    cfg.max_staleness_versions = 0;
    let mut n = Neuron::new(2, 0.0, [(1, 2.0)], cfg).unwrap();
    n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap();
    n.forward(forward(21, vec![FeedbackSource::Teacher])).unwrap();
    assert!(matches!(
        n.backward(feedback(20, FeedbackSource::Teacher, 1.0), 1_001).unwrap(),
        FeedbackStatus::Applied { .. }
    ));
    let updated = n.weight(1);
    assert_eq!(n.backward(feedback(21, FeedbackSource::Teacher, 1.0), 1_002).unwrap(), FeedbackStatus::Stale);
    assert_eq!(n.weight(1), updated);
}

#[test]
fn unknown_contributor_and_exhausted_hops_do_not_train() {
    let mut n = Neuron::new(2, 0.0, [(1, 2.0)], config()).unwrap();
    n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap();
    let stranger = FeedbackSource::Neuron { neuron_id: 3, event_id: 30 };
    assert_eq!(n.backward(feedback(20, stranger, 1.0), 1_001).unwrap_err(), Error::UnexpectedContributor);
    let mut exhausted = feedback(20, FeedbackSource::Teacher, 1.0);
    exhausted.hops_left = 0;
    assert_eq!(n.backward(exhausted, 1_001).unwrap_err(), Error::HopLimitExhausted);
    assert_eq!(n.weight(1), Some(2.0));
}

#[test]
fn pending_and_tombstones_share_a_bounded_budget() {
    let mut cfg = config();
    cfg.max_live_events = 1;
    let mut n = Neuron::new(2, 0.0, [(1, 2.0)], cfg).unwrap();
    n.forward(forward(20, vec![FeedbackSource::Teacher])).unwrap();
    assert_eq!(n.forward(forward(21, vec![FeedbackSource::Teacher])).unwrap_err(), Error::CapacityExceeded);
    assert!(matches!(
        n.backward(feedback(20, FeedbackSource::Teacher, 1.0), 1_001).unwrap(),
        FeedbackStatus::Applied { .. }
    ));
    assert_eq!(n.forward(forward(21, vec![FeedbackSource::Teacher])).unwrap_err(), Error::CapacityExceeded);
    let mut next = forward(21, vec![FeedbackSource::Teacher]);
    next.now_ms = 1_301;
    assert!(n.forward(next).is_ok());
}

#[test]
fn inference_does_not_keep_forward_context_or_accept_training() {
    let mut n = Neuron::new(2, 0.0, [(1, 2.0)], config()).unwrap();
    assert_eq!(n.forward(forward(20, vec![])).unwrap(), 6.0);
    assert!(n.trace(20).is_none());
    assert_eq!(n.backward(feedback(20, FeedbackSource::Teacher, 1.0), 1_001).unwrap(), FeedbackStatus::IgnoredDuplicate);
}
