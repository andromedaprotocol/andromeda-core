# Overview

The Economics ADO allows users to deposit funds to be used to pay fees implemented on ADO actions (Execute Messages) by the ADODB. Deposited funds can be either native funds such as "uandr" or CW20 tokens where the contract address is used. The fees are automatically called by the ADO that implements them. 

Fees are charged in the following order:
- ADO: First, the ADO requesting the fees is checked for funds and if available will use these funds to pay the fee.
- App: The App contract of the ADO requesting the fees.
- Payee: The address that sent the message to the ADO that is requesting the fees.

[Economics Full Documentation](https://docs.andromedaprotocol.io/andromeda/platform-and-framework/andromeda-messaging-protocol/economics-engine)