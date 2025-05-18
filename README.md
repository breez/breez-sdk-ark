# Breez SDK - Nodeless *(Ark Implementation)*

## **Overview**

The Breez SDK provides developers with a end-to-end solution for integrating self-custodial Lightning payments into their apps and services. It eliminates the need for third parties, simplifies the complexities of Bitcoin and Lightning, and enables seamless onboarding for billions of users to the future of peer-to-peer payments.

To provide the best experience for their end-users, developers can choose between the following implementations:

- [Breez SDK -  Nodeless *(Liquid Implementation)*](https://sdk-doc-liquid.breez.technology/)
- [Breez SDK - Nodeless *(Spark Implementation)*](https://sdk-doc-spark.breez.technology/)
- [Breez SDK - Nodeless *(Ark Implementation)*](https://sdk-doc-ark.breez.technology/)
- [Breez SDK - Native *(Greenlight Implementation)*](https://sdk-doc.breez.technology/)


**The Breez SDK is free for developers.**

## **What Is the Breez SDK - Nodeless *(Ark Implementation)*?**

It’s an Ark integration that offers a self-custodial, end-to-end solution for integrating Lightning payments, utilizing the Ark Network with on-chain interoperability and third-party fiat on-ramps. Using the SDK you'll able to:

- **Send payments** via various protocols such as: Bolt11, Bolt12, BIP353, LNURL-Pay, Lightning address, BTC address
- **Receive payments** via various protocols such as: Bolt11, LNURL-Withdraw, LNURL-Pay, Lightning address, BTC address
  
**Key Features**

- [x] Send and receive inside Ark
- [x] On-chain interoperability
- [ ] Send and receive Lightning payments 
- [ ] Complete LNURL functionality
- [ ] Multi-app support
- [ ] Multi-device support
- [ ] Real-time state backup
- [x] Keys are only held by users
- [ ] Built-in fiat on-ramp
- [x] Free open-source solution


## Getting Started 

Head over to the [Breez SDK - Nodeless *(Ark Implementation)* documentation](https://sdk-doc-ark.breez.technology/) to start implementing Lightning in your app.

You'll need an API key to use the Breez SDK - Spark *(Ark Implementation)*. To request an API key is free — you just need to [complete this simple form.](https://breez.technology/request-api-key/#contact-us-form-sdk)

## **API**

API documentation is [here](https://breez.github.io/breez-sdk-ark/breez_sdk_ark/).

## **Command Line**

The [Breez SDK - Nodeless *(Ark Implementation)* cli](https://github.com/breez/breez-sdk-ark/tree/main/cli) is a command line client that allows you to interact with and test the functionality of the SDK.

## **Support**

Have a question for the team? Join our [Telegram channel](https://t.me/breezsdk) or email us at [contact@breez.technology](mailto:contact@breez.technology) 

## How Does Nodeless *(Spark Implementation)* Work?

## **Build & Test**

- **cli**:  Contains the Rust command line interface client for the SDK - *Ark*.
- **lib**: Contains the root Rust cargo workspace.
    - **bindings**: The ffi bindings for Kotlin, Flutter, Python, React Native, and Swift.
    - **core**: The core SDK - *Ark* rust library.
- **packages**: Contains the plugin packages for Dart, Flutter, and React Native.

Within each sub-project readme, there are instructions on how to build, test, and run.

## **Contributing**

Contributions are always welcome. Please read our [contribution guide](CONTRIBUTING.md) to get started.

## **SDK Development Roadmap**

- [x]  Send and receive inside Ark
- [ ]  Send/Receive Lightning payments
- [x]  CLI Interface
- [ ]  Foreign languages bindings
- [ ]  Export/Import SDK data
- [x]  Pay BTC on-chain
- [x]  Receive via on-chain address
- [ ]  LNURL-Pay
- [ ]  LNURL-Withdraw
- [ ]  Send to a Lightning address
- [ ]  Receive via Lightning address
- [ ]  Webhook for receiving payments
- [ ]  Offline receive via notifications
- [ ]  Offline swaps via notifications
- [ ]  Real-time sync
- [ ]  External input parsers
- [ ]  Bolt12 send
- [ ]  BIP353 pay codes
- [ ]  Bolt12 receive
- [ ]  WebAssembly 
