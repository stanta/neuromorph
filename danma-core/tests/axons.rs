use danma_core::{
    derived_event_id, Activation, Axon, Config, ForwardSignal, Neuron, SignalStatus,
    SynapticInput,
};

fn config() -> Config {
    Config {
        activation: Activation::Linear,
        learning_rate: 0.1,
        activation_ttl_ms: 1_000,
        replay_retention_ms: 1_000,
        max_live_events: 64,
        max_staleness_versions: 8,
    }
}

#[test]
fn axon_fanin_waits_for_all_dendrites_and_deduplicates() {
    let mut neuron = Neuron::new_with_axons(
        3,
        0.0,
        [(1, 2.0), (2, 3.0)],
        [Axon { edge_id: 34, to: 4 }],
        config(),
    )
    .unwrap();

    assert_eq!(neuron.axons(), &[Axon { edge_id: 34, to: 4 }]);

    let event_id = derived_event_id(77, 3);
    let first = ForwardSignal {
        event_id,
        trace_id: 77,
        now_ms: 10,
        input: SynapticInput {
            from: 1,
            source_event_id: 11,
            value: 1.0,
        },
        training: false,
    };
    assert_eq!(
        neuron.receive_signal(first).unwrap(),
        SignalStatus::Pending { remaining: 1 }
    );
    assert_eq!(
        neuron.receive_signal(first).unwrap(),
        SignalStatus::IgnoredDuplicate
    );

    let second = ForwardSignal {
        event_id,
        trace_id: 77,
        now_ms: 11,
        input: SynapticInput {
            from: 2,
            source_event_id: 22,
            value: 2.0,
        },
        training: false,
    };
    assert_eq!(
        neuron.receive_signal(second).unwrap(),
        SignalStatus::Fired { output: 8.0 }
    );
    assert_eq!(
        neuron.receive_signal(second).unwrap(),
        SignalStatus::IgnoredDuplicate
    );
}

#[test]
fn training_signal_expects_feedback_from_downstream_axon() {
    use danma_core::{Feedback, FeedbackSource, FeedbackStatus};

    let mut neuron = Neuron::new_with_axons(
        3,
        0.0,
        [(1, 2.0)],
        [Axon { edge_id: 34, to: 4 }],
        config(),
    )
    .unwrap();

    let event_id = derived_event_id(88, 3);
    assert_eq!(
        neuron
            .receive_signal(ForwardSignal {
                event_id,
                trace_id: 88,
                now_ms: 100,
                input: SynapticInput {
                    from: 1,
                    source_event_id: 10,
                    value: 2.0,
                },
                training: true,
            })
            .unwrap(),
        SignalStatus::Fired { output: 4.0 }
    );
    assert_eq!(neuron.trace(event_id).unwrap().expected_contributions, 1);

    let child_event = derived_event_id(88, 4);
    let status = neuron
        .backward(
            Feedback {
                event_id,
                from: FeedbackSource::Neuron {
                    neuron_id: 4,
                    event_id: child_event,
                },
                gradient: 1.0,
                expires_at_ms: 900,
                hops_left: 2,
            },
            101,
        )
        .unwrap();
    assert!(matches!(status, FeedbackStatus::Applied { version: 1, .. }));
}
