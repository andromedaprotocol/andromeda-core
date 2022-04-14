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

Randomness: Ideally terrand can provide a random number in the range of the number of available NFTs for sale, we'll then use that number as the index of the vector which holds all the NFTs. 

## Currently Tackling
Mint function (which may or may not be desired in the first place)
Terrand (Limited documentation)
