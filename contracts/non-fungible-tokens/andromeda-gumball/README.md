# The Gumball of the 2020s

Send funds, receive a random NFT

## Basic Functionality

Instantiating the contract sets the cw-721 contract's address

There are 2 statuses:
Available (represented by "true" status)
Refilling (represented by "false" status)

Available: halts the mint function and allows the buy function.
Refilling: halts the buy function and allows the mint function.

Switch State: The function used to set the price, recipient, status, and max amount per wallet.

## Currently Tackling

Looking for a random number generator on Juno
