# Luna Arbitrage Wallet

### Personas
- **trader**: The person/wallet who will be performing the arbitrage of funds. Earns a configurable % of profits.
- **funder**: The perso/wallet that supplies the funds that will be arbitraged.

### Contract Lifecycle
- The **trader** instantiates the contract, supplying an address for the **funder**.
- The **trader** and the **funder** are both able to update the state of the contract. They can/should update the following:
    - The ***whitelist*** of addresses that the **funder** will be allowed to send funds to. These should be exchange addresses.
    - The ***assets*** that will be considered part of the arbitrage. These should all be assets of roughly equivalent value, such as Luna/cLuna/bLuna, etc.
    - The ***commission*** amount of profits that will be allocated to the trader.
    - The ***trader_withdrawal_address*** (adjustable only by trader) that the trader's funds will be withdrawn to, if different from the address submitting transactions.
- The **funder** or **trader** (or both) locks the contract. Once the contract has been locked, the state (described above) can no longer be modified until it is unlocked by everyone who has locked it.
- The **funder** deposits funds into the contract. The amount of funds deposited is kept track of.
- The **trader** can now, from their own wallet, send coins/tokens/msgs to the smart contract that will be forwarded on accordingly. This gives the **trader** the ability to freely interact with the wallet's funds, but only when sending to the whitelisted addresses.
- At any time, the **funder** or **trader** can withdraw funds. The **trader** receives a % of the ***profit*** that has been made (default 20%). The rest of the funds are withdrawable by the **funder**.

### Future Considerations / Todo
- Use submessages to track that assets being received from trade messages are part of the approved assets list. If not, the transaction will be rejected. This is to prevent the **trader** from discreetly exchanging assets for an un-approved asset through a contract that allows trade to multiple currencies - such as the way that PRISM is configured.
