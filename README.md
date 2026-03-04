# iot‑flow
**A DSL that turns sensor‑to‑actuator pipelines into safe, zero‑runtime Rust/embedded C**

Built by **Myroslav Mokhammad Abdeljawwad** to solve the pain of wiring IoT devices without drowning in boilerplate.

---

## ✨ Features

- Domain‑specific language for expressing sensor → actuator pipelines
- Compile‑time safety guarantees – no runtime interpreter needed
- Generates idiomatic Rust and optional embedded C backends
- Zero‑runtime overhead: the generated code is pure, hand‑written
- Extensible via a small, well‑documented parser & code generator API
- Includes a minimal runtime for managing interrupts and I/O on bare metal

---

## 🚀 Installation

```bash
# Clone the repo
git clone https://github.com/<your‑handle>/iot-flow.git
cd iot-flow

# Build the library (requires Rust 1.70+)
cargo build --release
```

The crate is available on crates.io; add it to your `Cargo.toml`:

```toml
[dependencies]
iot-flow = "0.1"
```

---

## 📦 Usage

### 1️⃣ Define a pipeline in `.flow`

Create an `examples/basic.flow` file (already included):

```flow
// examples/basic.flow
sensor temperature from /dev/ttyUSB0 {
    baud: 115200,
}
actuator fan {
    pin: 17,
}

pipeline temp_to_fan {
    input: temperature;
    output: fan;
    rule: if value > 30 { activate(); } else { deactivate(); }
}
```

### 2️⃣ Compile the flow file

```bash
# Generates Rust source under target/generated/
cargo run --bin iot-flow-cli examples/basic.flow
```

The CLI will emit `generated.rs` which you can include in your project:

```rust
mod generated {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}
```

### 3️⃣ Run the generated code

```bash
# Assuming the generated module exposes a `run()` function
fn main() {
    generated::run();
}
```

> **Tip:** The library ships with a lightweight runtime (`src/runtime/runtime.rs`) that handles serial I/O and GPIO. See the docs for customizing it.

---

## 🤝 Contributing

I’m always excited to see new ideas! If you’d like to help:

1. Fork the repo
2. Create a feature branch (`git checkout -b feat/your-feature`)
3. Write tests in `tests/integration_test.rs`
4. Run `cargo test` and ensure everything passes
5. Open a pull request

Feel free to open issues for bugs, enhancements, or just questions—Myroslav Mokhammad Abdeljawwad is happy to chat.

---

## 📜 License

MIT – see the [LICENSE](LICENSE) file.

---

## 🌐 See Also

- **Inspiration**:  
  [Building Forge Mods in 1.20.x: Practical Notes by Myroslav Mokhammad Abdeljawwad](https://dev.to/myroslavmokhammadabd/building-forge-mods-in-120x-practical-notes-by-myroslav-mokhammad-abdeljawwad-3ig0)

---