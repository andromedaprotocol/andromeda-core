# Andromeda Shunting ADO

## Introduction

The `Andromeda Shunting ADO` is a dedicated smart contract designed for the evaluation of mathematical expressions.

## Why Shunting ADO

Many smart contracts involve intricate core logic, particularly in the realm of complex financial models. The Andromeda Shunting ADO serves multiple purposes:

1. <b>Simplified Logic Implementation:</b> Rather than embedding intricate financial models directly into the smart contract, this ADO allows users to express various equations as `expressions` and query the smart contract with parameters to obtain results. This streamlined approach facilitates the rapid development of demo projects with customized logic.

2. <b>Seamless Integration with Other Smart Contracts:</b> Integration with other smart contracts is often a prerequisite for financial models. The Andromeda Shunting ADO makes it easy to interact with other smart contracts. By passing parameters for third-party smart contract calls, users can incorporate the results into their mathematical expressions. This feature is particularly useful for incorporating market prices from oracles into financial models for considerations such as price impact and rebalancing factors.

3. <b>Flexible Model Adjustments:</b> In instances where adjustments to financial models become necessary, users employing this ADO need not undergo the cumbersome process of upgrading and redeploying the smart contract. Instead, they can simply update the expression, allowing the smart contract to adapt to the revised logic without the need for redeployment.

## InstantiateMsg

```rust
pub struct InstantiateMsg {
    pub expressions: Vec<String>,
    pub kernel_address: String,
    pub owner: Option<String>
}
```
```json
{
    "expressions": ["...", ...],
    "kernel_address": "...",
}
```

The `expressions` field is provided as an array of mathematical expressions. For example:

```
[
    "{x0} ^ 2",
    "{y0} * x1"
]
```

In the expression, `{x<n>}` means the nth parameter in the params of the query, and `{y<n>}` means the result of the nth expression. In the above example, when evaluating the expression, the first expression calculate the square of the first parameter passed by the query and the second expression get the first expression's result and multiply it by second parameter. Only the last expression's result is returned as a query result. 

_**Warning:** Expressions can only use the previous expression's result._


## ExecuteMsg
_**Warning:** Only owner or operator can update expressions._
```rust
pub enum ExecuteMsg {
    UpdateExpressions { expressions: Vec<String> },
}
```
```json
{
    "update_expressions":  {
        "expressions": ["...", ...] 
    },
}
```

Expressions can be updated using the `ExecuteMsg`.

## QueryMsg
```rust
pub enum QueryMsg {
    #[returns(ShuntingResponse)]
    Evaluate { params: Vec<EvaluateParam> },
}
```
```json
{
    "evaluate":  {
        "params": [] 
    },
}
```

We use `Evaulate` msg to evaluate the mathematical expressions. It returns a `ShuntingResponse` message which is structured as follows. 

```rust
pub struct ShuntingResponse {
    pub result: String,
}
```
```json
{
    "result":  "",
}
```


The `params` of the `Evaluate` message can be either raw value or references to other smart contracts.

```rust
pub enum EvaluateParam {
    Value(String),
    Reference(EvaluateRefParam),
}
```
- `Value(String)` is used to pass raw(numerical) values.
- `Reference(EvaluateRefParam)` is used to convey the result of another contract.

For instance, if you wish to pass the values 2 and 3.14 as parameters, the following message can be used:

```rust
Evaluate {
    params: vec![
        EvaluateParam::Value("2".to_string()),
        EvaluateParam::Value("3.14".to_string()),
    ]
}
```
```json
{
    "evaluate": {
        "params":[{"value":"2"}, {"value":"3.14"}]
    }
}
```

## EvaluateRefParam

```rust
pub struct EvaluateRefParam {
    pub contract: Addr,
    pub msg: String,
    pub accessor: String,
}
```
- `contract` is the address of the contract intended for integration.
- `msg` is a `base64` encoded message slated for dispatch to the contract.
- `accessor` identifies the field within the result that is intended for use as a parameter.

For instance, if there is a need to invoke another Shunting ADO contract with the provided example, the following message can be employed:

```json
{
    "evaluate":{
        "params":[
            {
                "reference": {
                    "contract":"...",
                    "msg":"ewogICAgImV2YWx1YXRlIjogewogICAgICAgICJwYXJhbXMiOlt7InZhbHVlIjoiMiJ9LCB7InZhbHVlIjoiMy4xNCJ9XQogICAgfQp9Cg==",
                    "accessor":"result"
                }
            }
        ]
    }
}
```
In this example, as the Shunting ADO returns a `ShuntingResponse` and the intention is to use the `"result"` field of the query response as a parameter, the `accessor` is specified as `"result"`. The `msg` is obtained through the base64 encoding of the aforementioned evaluate message.

If the integrated smart contract yields a complex data structure, it may be necessary to utilize nested fields. For instance, consider a scenario where the returned value resembles the following.
```json
{
    "parent1": {
        "parent2": {
            "result": "value"
        }
    }
}
```

In this case to use `"result"` of the `"parent2"`, the `accessor` can be defined as `"parent1.parent2.result"`.