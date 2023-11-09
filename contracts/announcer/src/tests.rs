use cosmwasm_std::testing::{mock_dependencies, mock_info, mock_env};

use crate::{contract::instantiate, msg::InstantiateMsg};

#[test]
fn successful_instantiation() {
    let mut deps = mock_dependencies();
    let instantiate_msg = InstantiateMsg {
    };

    let info = mock_info("sender", &[]);
    let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
    assert_eq!(0, res.messages.len());
}

