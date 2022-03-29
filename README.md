# Luna Arbitrage Wallet

### Personas
- **owner**: The person/wallet who will be performing the arbitrage of funds. Earns a configurable % of profits.
- **user**: The perso/wallet that supplies the funds that will be arbitraged.

### Contract Lifecycle
- The **owner** instantiates the contract, supplying an address for the **user**.
- The **user** whitelists specific addresses that the **owner** will be able to interact with - most likely, these should be exchange addresses. (Note: until the contract has been 'locked', the owner can supply these as well).
- The **user** configures the list of Cw20 addresses & denoms that will be arbitraged with - these should be currencies that are all considered to have essentially equivalent value, such as Luna/bLuna/cLuna.
- The **user** 'locks' the contract. This prevents the **owner** from making further configuration changes, securing the **user**'s funds.
- The **user** deposits funds into the contract. The amount of funds deposited is kept track of.
- The **owner** can now, from their own wallet, send tokens/messages/Cw20ExecuteMsg's to the smart contract that will be forwarded on accordingly. This gives the owner the ability to freely interact with the wallet's funds, but only with the whitelisted addresses.
- At any time, the **owner** or **user** can withdraw funds. The **owner** receives a % of the _profit_ that has been made (default 20%). The rest of the funds are withdrawable by the user.



NOTE: I probably should have flipped the naming of user/owner as I think the current naming scheme feels backwards.
