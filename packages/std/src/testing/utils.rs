use cosmwasm_std::Response;

pub fn assert_response(response: &Response, expected: &Response, test_name: &str) {
    for attr in &expected.attributes {
        assert!(
            response.attributes.contains(attr),
            "Attribute {:?} not found in {}",
            attr,
            test_name
        );
    }
    for event in &expected.events {
        assert!(
            response.events.contains(event),
            "Event {:?} not found in {}",
            event,
            test_name
        );
    }
    for msg in &expected.messages {
        assert!(
            response.messages.contains(msg),
            "Message {:?} not found in {}",
            msg,
            test_name
        );
    }
}
