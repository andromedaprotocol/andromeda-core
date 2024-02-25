use enum_repr::EnumRepr;

#[EnumRepr(type = "u64")]
pub enum ReplyId {
    Cw20WithdrawMsg = 999,
    PayFee = 9999,
}
