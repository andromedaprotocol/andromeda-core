use enum_repr::EnumRepr;

#[EnumRepr(type = "u64")]
pub enum ReplyId {
    // Kernel
    AMPMsg = 100,
    CreateADO = 101,
    UpdateOwnership = 102,
    IBCHooksPacketSend = 103,
    Recovery = 104,
    RegisterUsername = 105,
    IBCTransfer = 106,
    SendFundsWithMsg = 107,
    // App
    ClaimOwnership = 200,
    AssignApp = 201,
    RegisterPath = 202,
    CrossChainCreate = 203,
    // Economics
    Cw20WithdrawMsg = 300,
    PayFee = 301,
}
