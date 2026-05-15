# 🏥 Medical Research Feed

A medical research discovery engine powered by the **X-Algorithm** (open-sourced by Elon Musk / xAI).

It searches [PubMed](https://pubmed.ncbi.nlm.nih.gov/) — the world's largest medical research database — and surfaces both papers you'd expect **and** cross-discipline papers you didn't know you needed.

---

## 💡 What It Does

An oncologist types `"oncology cancer treatment"`.

The system does two things:

| Step | Name | What happens |
|------|------|--------------|
| ⚡ Thunder | In-network | Searches PubMed directly for that topic |
| 🔮 Phoenix | Out-of-network | Extracts signals from top results, searches again from a different angle |
| 📊 Merge | Final feed | Combines both, scores by relevance, shows top 15 ranked papers |

Each paper links directly to PubMed and opens in a new tab.

---

## 🧠 How the X-Algorithm Is Used

This project borrows three concepts from [xAI's open-source algorithm](https://github.com/xai-org/x-algorithm):

- **Phoenix Retrieval** — discovers content outside your direct search
- **In-network + Out-of-network** — separates direct results from discovered ones
- **Candidate Pipeline** — fetch → score → rank → serve

Originally built for Twitter/X posts. Here it's applied to medical research papers.

---

## 🖥️ UI Features

- Medical-themed design (deep blue + green, Georgia serif font)
- Animated loading screen with DNA spinner and live step-by-step progress
- Cards fade and slide in as results load
- Animated relevance score bar on each card
- `IN-NETWORK` and `DISCOVERY` badges so you know where each paper came from
- Inline search bar on results page — no need to go back
- Fully responsive on mobile

---

## 🚀 Running Locally

**Requirements:** [Rust](https://rustup.rs/) installed

```bash
# Clone the repo
git clone https://github.com/aposalik/medical-feed.git
cd medical-feed

# Run
cargo run
```

Then open your browser at:

```
http://localhost:3000
```

Type any medical topic and hit Search. That's it.

---

## 🔍 Example Searches

```
oncology cancer treatment
alzheimer neurodegeneration
diabetes insulin resistance
breast cancer immunotherapy
cardiovascular disease prevention
```

---

## 📁 Project Structure

```
medical-feed/
├── src/
│   └── main.rs       # entire app — pipeline + web server + UI
├── Cargo.toml        # dependencies
└── README.md
```

Everything lives in `main.rs` — the pipeline logic, the web server (Axum), and the HTML/CSS/JS UI are all in one file, making it easy to follow end to end.

---

## 🛠️ Tech Stack

| Tool | Purpose |
|------|---------|
| [Rust](https://www.rust-lang.org/) | Language |
| [Axum](https://github.com/tokio-rs/axum) | Web server |
| [Tokio](https://tokio.rs/) | Async runtime |
| [Reqwest](https://github.com/seanmonstar/reqwest) | HTTP client |
| [Serde JSON](https://github.com/serde-rs/json) | JSON parsing |
| [PubMed API](https://www.ncbi.nlm.nih.gov/home/develop/api/) | Research paper data |

---

## 📖 How the Pipeline Works (Simple Version)

```
User types topic
      ↓
Step 1 — Search PubMed → get 20 paper IDs
      ↓
Step 2 — Fetch full details (title, abstract, journal)
      ↓
Step 3 — Score each paper by keyword match (0–100%)
      ↓
Step 4 — Phoenix: extract new terms from top papers → search PubMed again
      ↓
Step 5 — Merge both lists, deduplicate, re-rank → show top 15
```

---

## 🙏 Credits

- Algorithm concept: [xAI / x-algorithm](https://github.com/xai-org/x-algorithm)
- Paper data: [PubMed / NCBI](https://pubmed.ncbi.nlm.nih.gov/)
