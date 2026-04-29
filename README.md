### Anchor Voting Logic 

In this file we have a code that is written on solana blockchain and is deployable on the devnet of solana. This is a voting application which takes initializes
votes, then initializes the options and finally let's you vote for your candidate. How does this work.

**Anchor** writes most of the boilerplate therefore, this program is not optimized for compute units. However, one can see how instructions work. We give state 
to the program using **#[account]**. Then under the **#[derive(Accounts)]** you have all your accounts, which will be used when running the program. For this
program we have assigned user as the person running this program locally. For the user to authorize transactions, you must genrate 
``` solana-keygen grid --starts-with vot:1 --ignore-case ``` and store it in you device. This will give you a .json file which has your public and private keys.
The the Poll Account and Candidate Accounts are Program Derived Addresses (PDA's). This needs to be a PDA because we don't want to give anyone the authority to 
make changes to our poll and the candidate info. This let's one trust the voting system because this way it stays tamper evident.

### Testing the function

We are using **LiteSVM** framework to quickly test the functions. Right now we have only tested initiaze poll and initialize candidate. We need to test the 
voting function and the edge cases in case some tries to vote before the start of poll or after the poll has ended. They will be added. When using litesvm you 
need to assign a payer which we do by generating a new keypair, then we add SOLs to the keypair so the user can initialize poll and candidate. Then, we also 
need to provide instructions which is in the form of bytes. Then finally we send transactions and test with assert! macro.

### Using this locally

1. Clone this repository

2. Make changes to the deploy .json, Anchor.toml, lib.rs. Here put your address that your program has. You can run the keygen command in your project folder so 
new .json keypair file is created in same folder then you can use cp .json /taget/deploy/.json to change the bytes.

            anchor build

            anchor program deploy

3. These bash commands give you the metadata and idl for you program, then you can go to explorer.solana.com . Choose devnet option and then paste your id from above

4. For local testing run the following

            cargo test

5. If you have a println! macro in your test code then you have to run the following script

            cargo test -- --nocapture

### More Stuff

I will be adding more tests to the program. Then, I also plan to add a frontend to this.