# ICV - Your AI Career Coach on ICP Smart Contract

An **LLM-powered AI career coach** running on **Internet Computer Protocol (ICP)** smart contracts. This project provides job seekers with expert career guidance through a **chat interface**, helping with resume reviews, interview prep, salary negotiation, and more.

![LLM Chatbot](https://icp.ninja/examples/_attachments/llm_chatbot.png)

## Tech Stack

### Backend

- **ICP Canister (Smart Contract)** – Hosts AI logic and interacts with the LLM.
- **Llama 3.1:8b (via Ollama)** – The underlying AI model powering career coaching.
- **Rust** – Implements backend logic, prompt engineering, and smart contracts.

### Frontend

- **React 19** – Client-side UI.
- **TailwindCSS + ShadCN** – Modern and stylish UI components.
- **Zustand** – State management for chat and user sessions.

---

## Features

- **AI-Powered Career Coaching:** Get expert guidance on job search, resume optimization, and salary negotiation.
- **Chat Interface:** Intuitive, real-time conversation with a familiar chat UI.
- **Personalized with Internet Identity** Connected with internet identity, you can get back to your conversation anytime.
- **Resume Upload & Analysis:** Upload your CV for AI-driven feedback.
- **Context-Aware Conversations:** Maintains chat history for continuous, coherent discussions.

---

## Project structure

The `/backend` folder contains the Rust smart contract:

- `Cargo.toml`, which defines the crate that will form the backend
- `lib.rs`, which contains the actual smart contract, and exports its interface
- `entities.rs`, which contains the entity models, on-chain stable data structure, and repositories
- `knowledge.rs`, which contains llm specific usecase

The `/frontend` folder contains web assets for the application's user interface. The user interface is written using the React framework.

---

## Getting Started

To build the project locally, follow these steps.

### 1. Clone the repository

```sh
git clone git@github.com:muhrifqii/ICV.git
```

### 2. Setting up Ollama

This project requires a running LLM model. To be able to test the agent locally, you'll need a server for processing the agent's prompts. For that, we'll use `ollama`, which is a tool that can download and serve LLMs.
See the documentation on the [Ollama website](https://ollama.com/) to install it. Once it's installed, run:

```
ollama serve
# Expected to start listening on port 11434
```

The above command will start the Ollama server, so that it can process requests by the agent. Additionally, and in a separate window, run the following command to download the LLM that will be used by the agent:

```
ollama run llama3.1:8b
```

The above command will download an 8B parameter model, which is around 4GiB. Once the command executes and the model is loaded, you can terminate it. You won't need to do this step again.

### 3. Install developer tools.

> Installing `dfx` natively is currently only supported on macOS and Linux systems. On Windows, you should run everything inside WSL, including the project itself.

> On Apple Silicon (e.g., Apple M1 chip), make sure you have Rosetta installed (`softwareupdate --install-rosetta`).

1. Install `dfx` with the following command:

   ```
   sh -ci "$(curl -fsSL https://internetcomputer.org/install.sh)"
   ```

1. [Install NodeJS](https://nodejs.org/en/download/package-manager) or use dev tools such as `nvm`.

1. Install [Rust](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo): `curl https://sh.rustup.rs -sSf | sh`

1. Install [candid-extractor](https://crates.io/crates/candid-extractor): `cargo install candid-extractor`

Lastly, navigate into the project's directory.

### 4. Create a local developer identity.

To manage the project's canisters, it is recommended that you create a local [developer identity](https://internetcomputer.org/docs/building-apps/getting-started/identities) rather than use the `dfx` default identity that is not stored securely.

To create a new identity, run the commands:

```
dfx start --background

dfx identity new IDENTITY_NAME

dfx identity use IDENTITY_NAME
```

Replace `IDENTITY_NAME` with your preferred identity name. The first command `dfx start --background` starts the local `dfx` processes, then `dfx identity new` will create a new identity and return your identity's seed phase. Be sure to save this in a safe, secure location.

The third command `dfx identity use` will tell `dfx` to use your new identity as the active identity. Any canister smart contracts created after running `dfx identity use` will be owned and controlled by the active identity.

Your identity will have a principal ID associated with it. Principal IDs are used to identify different entities on ICP, such as users and canisters.

[Learn more about ICP developer identities](https://internetcomputer.org/docs/building-apps/getting-started/identities).

### 5. Deploy the project locally.

Deploy your project to your local developer environment with the command:

```
dfx deploy
```

Your project will be hosted on your local machine. The local canister URLs for your project will be shown in the terminal window as output of the `dfx deploy` command. You can open these URLs in your web browser to view the local instance of your project.

---

## License

This project is licensed under the Apache 2.0 License

## Contributing

This project is currenly in alpha, bootstraped for a hackathon, and is not yet open for contributions.

## Show your support

If you find this project useful and exciting, let me know by starring this repository ⭐️
