use axum::{extract::Query, response::Html, routing::get, Router};
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Paper {
    id: String,
    title: String,
    abstract_text: String,
    journal: String,
}

#[derive(Debug)]
struct ScoredPaper {
    paper: Paper,
    score: f32,
    source: &'static str,
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/search", get(search_handler));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running at http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler() -> Html<String> {
    Html(render_index())
}

async fn search_handler(Query(params): Query<HashMap<String, String>>) -> Html<String> {
    let topic = params.get("q").map(|s| s.as_str()).unwrap_or("").trim().to_string();

    if topic.is_empty() {
        return Html(render_index());
    }

    let ids = fetch_paper_ids(&topic).await;
    let papers = fetch_paper_details(&ids).await;
    let ranked = score_and_rank(papers, &topic, "IN-NETWORK");
    let phoenix_papers = phoenix_retrieval(&ranked).await;
    let final_feed = merge_and_rank(ranked, phoenix_papers, &topic);

    Html(render_results(&topic, &final_feed))
}

// ── HTML Rendering ────────────────────────────────────────────────────────────

fn render_index() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
    <title>Medical Research Feed</title>
    {css}
</head>
<body>
    <div class="hero">
        <div class="cross pulse">✚</div>
        <h1 class="fade-in-down">Medical Research Feed</h1>
        <p class="subtitle fade-in-up">Powered by <span class="badge">X-Algorithm</span> · Thunder + Phoenix Retrieval</p>
        <form action="/search" method="get" class="search-form fade-in-up" id="searchForm" onsubmit="handleSubmit(event)">
            <div class="input-wrapper">
                <input
                    type="text"
                    name="q"
                    id="searchInput"
                    placeholder="e.g. oncology cancer treatment..."
                    autofocus
                    oninput="handleInput(this.value)"
                    autocomplete="off"
                />
                <div class="input-hint" id="inputHint"></div>
            </div>
            <button type="submit" id="searchBtn">
                <span class="btn-text">Search</span>
                <span class="btn-icon">→</span>
            </button>
        </form>
        <p class="hint fade-in-up">Discovers both in-network and cross-discipline research papers</p>
        <div class="features fade-in-up">
            <div class="feature"><span>⚡</span> Thunder in-network retrieval</div>
            <div class="feature"><span>🔮</span> Phoenix out-of-network discovery</div>
            <div class="feature"><span>📊</span> X-Algorithm relevance scoring</div>
        </div>
    </div>

    <!-- Full-screen loading overlay (shown on submit) -->
    <div class="loading-overlay" id="loadingOverlay">
        <div class="loading-box">
            <div class="dna-spinner">
                <div class="dna-dot d1"></div>
                <div class="dna-dot d2"></div>
                <div class="dna-dot d3"></div>
                <div class="dna-dot d4"></div>
                <div class="dna-dot d5"></div>
            </div>
            <p class="loading-title" id="loadingTitle">Initialising Pipeline...</p>
            <p class="loading-sub" id="loadingSub">Connecting to PubMed</p>
            <div class="loading-progress">
                <div class="loading-bar" id="loadingBar"></div>
            </div>
            <div class="loading-steps">
                <div class="step" id="step1">⚡ Thunder: fetching in-network papers</div>
                <div class="step" id="step2">🔮 Phoenix: discovering cross-discipline research</div>
                <div class="step" id="step3">📊 X-Algorithm: scoring &amp; ranking</div>
                <div class="step" id="step4">✅ Building your personalised feed</div>
            </div>
        </div>
    </div>

    {js}
</body>
</html>"#,
        css = css(),
        js = js_index()
    )
}

fn render_results(topic: &str, feed: &[ScoredPaper]) -> String {
    let cards: String = feed.iter().enumerate().map(|(i, sp)| {
        let pubmed_url = format!("https://pubmed.ncbi.nlm.nih.gov/{}/", sp.paper.id);
        let badge_class = if sp.source == "IN-NETWORK" { "badge-network" } else { "badge-discovery" };
        let abstract_preview: String = sp.paper.abstract_text.chars().take(200).collect();
        let abstract_text = if abstract_preview.is_empty() || abstract_preview == "No abstract" {
            "Abstract not available for this paper.".to_string()
        } else {
            format!("{}...", abstract_preview)
        };
        let delay = i * 80;

        format!(
            r#"<a href="{url}" target="_blank" class="card-link" style="animation-delay:{delay}ms">
                <article class="card slide-in">
                    <div class="card-header">
                        <span class="rank">#{rank}</span>
                        <span class="source-badge {badge_class}">{source}</span>
                        <div class="score-bar-wrapper">
                            <div class="score-bar" style="--pct:{pct}%"></div>
                        </div>
                        <span class="score">{pct:.0}%</span>
                    </div>
                    <h2>{title}</h2>
                    <p class="journal">📄 {journal}</p>
                    <p class="abstract">{abstract_text}</p>
                    <span class="read-more">Read on PubMed →</span>
                </article>
            </a>"#,
            url = pubmed_url,
            delay = delay,
            rank = i + 1,
            badge_class = badge_class,
            source = sp.source,
            pct = sp.score * 100.0,
            title = sp.paper.title,
            journal = sp.paper.journal,
            abstract_text = abstract_text,
        )
    }).collect();

    let in_net = feed.iter().filter(|p| p.source == "IN-NETWORK").count();
    let discovery = feed.iter().filter(|p| p.source == "DISCOVERY").count();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1.0"/>
    <title>Results: {topic}</title>
    {css}
</head>
<body class="results-page">

    <header class="results-header">
        <a href="/" class="back">← New Search</a>
        <div class="header-center">
            <span class="cross-sm pulse">✚</span>
            <span class="header-title">Medical Research Feed</span>
            <span class="badge">X-Algorithm</span>
        </div>
        <div class="header-stats">
            <span class="stat-pill net">{in_net} Thunder</span>
            <span class="stat-pill disc">{discovery} Phoenix</span>
        </div>
    </header>

    <!-- inline search bar on results page -->
    <div class="results-search-bar">
        <form action="/search" method="get" class="inline-form" id="inlineForm" onsubmit="handleInlineSubmit(event)">
            <input type="text" name="q" value="{topic}" id="inlineInput" />
            <button type="submit" id="inlineBtn">
                <span class="btn-text">Search</span>
                <span class="btn-icon">→</span>
            </button>
        </form>
    </div>

    <!-- Loading overlay for re-search -->
    <div class="loading-overlay" id="loadingOverlay">
        <div class="loading-box">
            <div class="dna-spinner">
                <div class="dna-dot d1"></div>
                <div class="dna-dot d2"></div>
                <div class="dna-dot d3"></div>
                <div class="dna-dot d4"></div>
                <div class="dna-dot d5"></div>
            </div>
            <p class="loading-title" id="loadingTitle">Initialising Pipeline...</p>
            <p class="loading-sub" id="loadingSub">Connecting to PubMed</p>
            <div class="loading-progress">
                <div class="loading-bar" id="loadingBar"></div>
            </div>
            <div class="loading-steps">
                <div class="step" id="step1">⚡ Thunder: fetching in-network papers</div>
                <div class="step" id="step2">🔮 Phoenix: discovering cross-discipline research</div>
                <div class="step" id="step3">📊 X-Algorithm: scoring &amp; ranking</div>
                <div class="step" id="step4">✅ Building your personalised feed</div>
            </div>
        </div>
    </div>

    <main>
        <div class="results-meta fade-in-down">
            <h1>Results for: <em>"{topic}"</em></h1>
            <p>{count} papers found · <span class="badge-network">IN-NETWORK</span> from Thunder · <span class="badge-discovery">DISCOVERY</span> from Phoenix</p>
        </div>
        <div class="feed" id="feed">
            {cards}
        </div>
    </main>

    {js}
</body>
</html>"#,
        css = css(),
        js = js_results(),
        topic = topic,
        count = feed.len(),
        in_net = in_net,
        discovery = discovery,
        cards = cards,
    )
}

fn css() -> &'static str {
    r#"<style>
        *, *::before, *::after { margin: 0; padding: 0; box-sizing: border-box; }

        :root {
            --blue-dark: #0a3d62;
            --blue-mid: #1e6fa5;
            --blue-light: #56b4d3;
            --green-soft: #a8e6cf;
            --green-mid: #7dd4b0;
            --bg: #f0f4f8;
            --card-bg: #ffffff;
            --text: #1a2e3b;
            --text-muted: #4a6274;
        }

        body {
            font-family: 'Georgia', serif;
            background: var(--bg);
            color: var(--text);
            min-height: 100vh;
        }

        /* ── Animations ── */
        @keyframes fadeInDown {
            from { opacity: 0; transform: translateY(-24px); }
            to   { opacity: 1; transform: translateY(0); }
        }
        @keyframes fadeInUp {
            from { opacity: 0; transform: translateY(24px); }
            to   { opacity: 1; transform: translateY(0); }
        }
        @keyframes slideIn {
            from { opacity: 0; transform: translateX(-20px); }
            to   { opacity: 1; transform: translateX(0); }
        }
        @keyframes pulse {
            0%, 100% { transform: scale(1); opacity: 1; }
            50%       { transform: scale(1.18); opacity: 0.75; }
        }
        @keyframes barFill {
            from { width: 0; }
            to   { width: var(--pct); }
        }
        @keyframes dnaFloat {
            0%, 100% { transform: translateY(0px) scale(1); opacity: 1; }
            50%       { transform: translateY(-18px) scale(1.3); opacity: 0.6; }
        }
        @keyframes progressBar {
            0%   { width: 0%; }
            20%  { width: 25%; }
            45%  { width: 50%; }
            70%  { width: 75%; }
            90%  { width: 88%; }
            100% { width: 95%; }
        }
        @keyframes stepAppear {
            from { opacity: 0; transform: translateX(-12px); }
            to   { opacity: 1; transform: translateX(0); }
        }
        @keyframes overlayIn {
            from { opacity: 0; }
            to   { opacity: 1; }
        }
        @keyframes boxIn {
            from { opacity: 0; transform: translateY(30px) scale(0.95); }
            to   { opacity: 1; transform: translateY(0) scale(1); }
        }

        .fade-in-down { animation: fadeInDown 0.6s ease both; }
        .fade-in-up   { animation: fadeInUp  0.7s ease both 0.15s; }
        .slide-in     { animation: slideIn   0.5s ease both; }
        .pulse        { animation: pulse 2.2s ease-in-out infinite; }

        /* ── Hero ── */
        .hero {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            text-align: center;
            padding: 2rem;
            background: linear-gradient(160deg, #0a3d62 0%, #1e6fa5 60%, #56b4d3 100%);
            color: white;
        }

        .cross { font-size: 3rem; color: var(--green-soft); margin-bottom: 1rem; display: inline-block; }
        .cross-sm { font-size: 1.2rem; color: var(--green-soft); display: inline-block; }

        .hero h1 {
            font-size: 2.8rem;
            font-weight: 700;
            letter-spacing: 1px;
            margin-bottom: 0.5rem;
        }

        .subtitle { font-size: 1rem; opacity: 0.85; margin-bottom: 2.5rem; }

        .badge {
            background: var(--green-soft);
            color: var(--blue-dark);
            padding: 2px 10px;
            border-radius: 20px;
            font-size: 0.8rem;
            font-weight: bold;
            font-family: monospace;
        }

        /* ── Search Form ── */
        .search-form {
            display: flex;
            gap: 0.5rem;
            width: 100%;
            max-width: 600px;
        }

        .input-wrapper { position: relative; flex: 1; }

        .search-form input, .inline-form input {
            width: 100%;
            padding: 0.9rem 1.2rem;
            border: 2px solid transparent;
            border-radius: 8px;
            font-size: 1rem;
            font-family: 'Georgia', serif;
            outline: none;
            background: white;
            color: var(--text);
            transition: border-color 0.2s, box-shadow 0.2s;
        }

        .search-form input:focus, .inline-form input:focus {
            border-color: var(--green-soft);
            box-shadow: 0 0 0 3px rgba(168, 230, 207, 0.3);
        }

        .input-hint {
            position: absolute;
            bottom: -22px;
            left: 4px;
            font-size: 0.75rem;
            color: rgba(255,255,255,0.7);
            transition: opacity 0.2s;
            white-space: nowrap;
        }

        .search-form button, .inline-form button {
            padding: 0.9rem 1.8rem;
            background: var(--green-soft);
            color: var(--blue-dark);
            border: none;
            border-radius: 8px;
            font-size: 1rem;
            font-weight: bold;
            cursor: pointer;
            transition: background 0.2s, transform 0.1s;
            display: flex;
            align-items: center;
            gap: 6px;
        }

        .search-form button:hover, .inline-form button:hover {
            background: var(--green-mid);
            transform: translateY(-1px);
        }

        .search-form button:active, .inline-form button:active {
            transform: translateY(0) scale(0.97);
        }

        .btn-icon { transition: transform 0.2s; }
        button:hover .btn-icon { transform: translateX(4px); }

        .hint { margin-top: 1.8rem; font-size: 0.85rem; opacity: 0.65; }

        .features {
            display: flex;
            gap: 1.2rem;
            margin-top: 2rem;
            flex-wrap: wrap;
            justify-content: center;
            animation: fadeInUp 0.8s ease both 0.4s;
        }

        .feature {
            background: rgba(255,255,255,0.12);
            border: 1px solid rgba(255,255,255,0.2);
            border-radius: 20px;
            padding: 6px 14px;
            font-size: 0.82rem;
            display: flex;
            align-items: center;
            gap: 6px;
            backdrop-filter: blur(4px);
        }

        /* ── Loading Overlay ── */
        .loading-overlay {
            display: none;
            position: fixed;
            inset: 0;
            background: rgba(10, 61, 98, 0.92);
            z-index: 999;
            align-items: center;
            justify-content: center;
            animation: overlayIn 0.3s ease;
        }

        .loading-overlay.active {
            display: flex;
        }

        .loading-box {
            background: white;
            border-radius: 20px;
            padding: 2.5rem 3rem;
            text-align: center;
            max-width: 440px;
            width: 90%;
            animation: boxIn 0.4s ease;
        }

        /* DNA / dots spinner */
        .dna-spinner {
            display: flex;
            justify-content: center;
            align-items: flex-end;
            gap: 10px;
            height: 54px;
            margin-bottom: 1.4rem;
        }

        .dna-dot {
            width: 14px;
            height: 14px;
            border-radius: 50%;
            background: var(--blue-mid);
            animation: dnaFloat 1.2s ease-in-out infinite;
        }

        .dna-dot.d1 { animation-delay: 0s;    background: #0a3d62; }
        .dna-dot.d2 { animation-delay: 0.18s; background: #1e6fa5; }
        .dna-dot.d3 { animation-delay: 0.36s; background: #56b4d3; }
        .dna-dot.d4 { animation-delay: 0.54s; background: #7dd4b0; }
        .dna-dot.d5 { animation-delay: 0.72s; background: #a8e6cf; }

        .loading-title {
            font-size: 1.15rem;
            font-weight: bold;
            color: var(--blue-dark);
            margin-bottom: 0.3rem;
        }

        .loading-sub {
            font-size: 0.85rem;
            color: var(--text-muted);
            margin-bottom: 1.2rem;
            min-height: 20px;
        }

        .loading-progress {
            background: #e0edf5;
            border-radius: 10px;
            height: 6px;
            overflow: hidden;
            margin-bottom: 1.4rem;
        }

        .loading-bar {
            height: 100%;
            width: 0%;
            background: linear-gradient(90deg, var(--blue-mid), var(--green-soft));
            border-radius: 10px;
            animation: progressBar 8s ease forwards;
        }

        .loading-steps {
            display: flex;
            flex-direction: column;
            gap: 8px;
            text-align: left;
        }

        .step {
            font-size: 0.82rem;
            color: #aaa;
            padding: 6px 12px;
            border-radius: 8px;
            background: #f8f9fa;
            border-left: 3px solid #ddd;
            opacity: 0;
            transition: color 0.3s, border-color 0.3s, opacity 0.3s;
        }

        .step.active {
            color: var(--blue-dark);
            border-color: var(--blue-mid);
            opacity: 1;
            animation: stepAppear 0.4s ease;
        }

        .step.done {
            color: #0a5c3a;
            border-color: var(--green-mid);
            opacity: 1;
            background: #edfaf4;
        }

        /* ── Results Header ── */
        .results-header {
            background: var(--blue-dark);
            color: white;
            padding: 1rem 2rem;
            display: flex;
            align-items: center;
            justify-content: space-between;
            position: sticky;
            top: 0;
            z-index: 10;
            box-shadow: 0 2px 12px rgba(0,0,0,0.2);
        }

        .header-center {
            display: flex;
            align-items: center;
            gap: 0.6rem;
            font-size: 1rem;
            font-weight: bold;
        }

        .back {
            color: var(--green-soft);
            text-decoration: none;
            font-size: 0.9rem;
            transition: opacity 0.2s;
        }

        .back:hover { opacity: 0.7; }

        .header-stats { display: flex; gap: 0.5rem; }

        .stat-pill {
            font-size: 0.75rem;
            font-family: monospace;
            padding: 3px 10px;
            border-radius: 20px;
            font-weight: bold;
        }

        .stat-pill.net  { background: #d0eaff; color: var(--blue-dark); }
        .stat-pill.disc { background: #d4f5e9; color: #0a5c3a; }

        /* ── Inline search bar ── */
        .results-search-bar {
            background: #e4ecf3;
            padding: 0.8rem 2rem;
            border-bottom: 1px solid #c8dce8;
        }

        .inline-form {
            display: flex;
            gap: 0.5rem;
            max-width: 700px;
            margin: 0 auto;
        }

        .inline-form input {
            border-color: #c8dce8;
            background: white;
        }

        .inline-form button {
            padding: 0.7rem 1.4rem;
            font-size: 0.9rem;
        }

        /* ── Results Body ── */
        main {
            max-width: 860px;
            margin: 0 auto;
            padding: 2rem 1rem;
        }

        .results-meta {
            margin-bottom: 2rem;
            padding-bottom: 1rem;
            border-bottom: 2px solid #c8dce8;
        }

        .results-meta h1 { font-size: 1.5rem; margin-bottom: 0.4rem; color: var(--blue-dark); }
        .results-meta p  { font-size: 0.9rem; color: var(--text-muted); }

        /* ── Feed Cards ── */
        .feed { display: flex; flex-direction: column; gap: 1.2rem; }

        .card-link { text-decoration: none; color: inherit; }

        .card {
            background: var(--card-bg);
            border-radius: 12px;
            padding: 1.5rem;
            border-left: 5px solid var(--blue-mid);
            box-shadow: 0 2px 8px rgba(0,0,0,0.07);
            transition: transform 0.2s, box-shadow 0.2s, border-color 0.2s;
        }

        .card:hover {
            transform: translateY(-3px);
            box-shadow: 0 8px 24px rgba(0,0,0,0.13);
            border-color: var(--green-mid);
        }

        .card-header {
            display: flex;
            align-items: center;
            gap: 0.6rem;
            margin-bottom: 0.8rem;
        }

        .rank { font-size: 0.85rem; font-weight: bold; color: var(--blue-mid); font-family: monospace; }

        .source-badge {
            padding: 2px 10px;
            border-radius: 20px;
            font-size: 0.75rem;
            font-weight: bold;
            font-family: monospace;
        }

        .badge-network  { background: #d0eaff; color: var(--blue-dark); }
        .badge-discovery{ background: #d4f5e9; color: #0a5c3a; }

        /* relevance score bar */
        .score-bar-wrapper {
            flex: 1;
            height: 4px;
            background: #e0edf5;
            border-radius: 4px;
            overflow: hidden;
        }

        .score-bar {
            height: 100%;
            width: 0;
            background: linear-gradient(90deg, var(--blue-mid), var(--green-soft));
            border-radius: 4px;
            animation: barFill 0.8s ease forwards;
            animation-delay: inherit;
        }

        .score { font-size: 0.8rem; color: #7a9ab0; font-family: monospace; min-width: 36px; text-align: right; }

        .card h2 { font-size: 1.1rem; color: var(--blue-dark); margin-bottom: 0.4rem; line-height: 1.4; }
        .journal  { font-size: 0.82rem; color: #5a7a8a; margin-bottom: 0.6rem; font-style: italic; }
        .abstract { font-size: 0.88rem; color: #3a5060; line-height: 1.6; margin-bottom: 0.8rem; }
        .read-more{ font-size: 0.82rem; color: var(--blue-mid); font-weight: bold; transition: color 0.2s; }
        .card:hover .read-more { color: #0a5c3a; }

        /* ── Responsive ── */
        @media (max-width: 600px) {
            .hero h1 { font-size: 2rem; }
            .search-form { flex-direction: column; }
            .features { flex-direction: column; align-items: center; }
            .header-stats { display: none; }
            .results-header { padding: 0.8rem 1rem; }
            .loading-box { padding: 1.8rem 1.4rem; }
        }
    </style>"#
}

fn js_index() -> &'static str {
    r#"<script>
    const steps = [
        { id: 'step1', title: 'Fetching papers...', sub: 'Thunder: querying PubMed in-network', ms: 400 },
        { id: 'step2', title: 'Phoenix retrieval...', sub: 'Discovering cross-discipline research', ms: 2200 },
        { id: 'step3', title: 'Scoring with X-Algorithm...', sub: 'Predicting relevance & engagement', ms: 4200 },
        { id: 'step4', title: 'Building your feed...', sub: 'Merging & ranking results', ms: 6000 },
    ];

    function handleInput(val) {
        const hint = document.getElementById('inputHint');
        if (val.length > 2) {
            hint.textContent = '↵ Press Enter or click Search';
        } else {
            hint.textContent = '';
        }
    }

    function handleSubmit(e) {
        const input = document.getElementById('searchInput');
        if (!input.value.trim()) { e.preventDefault(); return; }
        showLoading();
    }

    function showLoading() {
        const overlay = document.getElementById('loadingOverlay');
        overlay.classList.add('active');
        let current = 0;

        function activateStep(i) {
            if (i >= steps.length) return;
            const s = steps[i];
            const el = document.getElementById(s.id);
            if (el) el.classList.add('active');
            document.getElementById('loadingTitle').textContent = s.title;
            document.getElementById('loadingSub').textContent = s.sub;
            if (i > 0) {
                const prev = document.getElementById(steps[i-1].id);
                if (prev) { prev.classList.remove('active'); prev.classList.add('done'); }
            }
        }

        steps.forEach((s, i) => {
            setTimeout(() => activateStep(i), s.ms);
        });
    }
    </script>"#
}

fn js_results() -> &'static str {
    r#"<script>
    // Animate cards as they enter viewport
    const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateX(0)';
            }
        });
    }, { threshold: 0.1 });

    document.querySelectorAll('.card-link').forEach(card => {
        observer.observe(card);
    });

    // Score bars — delay matches card delay
    document.querySelectorAll('.score-bar').forEach((bar, i) => {
        bar.style.animationDelay = (i * 80 + 300) + 'ms';
    });

    // Inline re-search loading
    const steps = [
        { id: 'step1', title: 'Fetching papers...', sub: 'Thunder: querying PubMed in-network', ms: 400 },
        { id: 'step2', title: 'Phoenix retrieval...', sub: 'Discovering cross-discipline research', ms: 2200 },
        { id: 'step3', title: 'Scoring with X-Algorithm...', sub: 'Predicting relevance & engagement', ms: 4200 },
        { id: 'step4', title: 'Building your feed...', sub: 'Merging & ranking results', ms: 6000 },
    ];

    function handleInlineSubmit(e) {
        const input = document.getElementById('inlineInput');
        if (!input.value.trim()) { e.preventDefault(); return; }
        const overlay = document.getElementById('loadingOverlay');
        overlay.classList.add('active');

        steps.forEach((s, i) => {
            setTimeout(() => {
                const el = document.getElementById(s.id);
                if (el) el.classList.add('active');
                document.getElementById('loadingTitle').textContent = s.title;
                document.getElementById('loadingSub').textContent = s.sub;
                if (i > 0) {
                    const prev = document.getElementById(steps[i-1].id);
                    if (prev) { prev.classList.remove('active'); prev.classList.add('done'); }
                }
            }, s.ms);
        });
    }
    </script>"#
}

// ── Pipeline ─────────────────────────────────────────────────────────────────

fn extract_phoenix_terms(ranked: &[ScoredPaper]) -> Vec<String> {
    let stopwords = vec![
        "the", "a", "an", "of", "in", "and", "or", "for", "to", "with",
        "on", "at", "by", "is", "are", "was", "be", "as", "from", "its",
        "cancer", "oncology", "treatment",
    ];

    let mut terms: Vec<String> = ranked
        .iter()
        .take(3)
        .flat_map(|sp| {
            sp.paper.title
                .split_whitespace()
                .map(|w| w.to_lowercase().trim_matches(|c: char| !c.is_alphabetic()).to_string())
                .filter(|w| w.len() > 4 && !stopwords.contains(&w.as_str()))
                .collect::<Vec<String>>()
        })
        .collect();

    terms.dedup();
    terms.truncate(3);
    terms
}

async fn phoenix_retrieval(ranked: &[ScoredPaper]) -> Vec<Paper> {
    let terms = extract_phoenix_terms(ranked);
    let mut all_ids: Vec<String> = vec![];
    for term in &terms {
        let ids = fetch_paper_ids(term).await;
        all_ids.extend(ids);
    }
    all_ids.sort();
    all_ids.dedup();
    fetch_paper_details(&all_ids).await
}

async fn fetch_paper_ids(topic: &str) -> Vec<String> {
    let url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}&retmax=20&retmode=json",
        topic.replace(" ", "+")
    );
    let Ok(response) = reqwest::get(&url).await else { return vec![]; };
    let Ok(json) = response.json::<serde_json::Value>().await else { return vec![]; };
    json["esearchresult"]["idlist"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|id| id.as_str().unwrap_or("").to_string())
        .collect()
}

fn score_and_rank(papers: Vec<Paper>, topic: &str, source: &'static str) -> Vec<ScoredPaper> {
    let keywords: Vec<String> = topic.split_whitespace().map(|w| w.to_lowercase()).collect();
    let mut scored: Vec<ScoredPaper> = papers
        .into_iter()
        .map(|paper| {
            let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();
            let matches = keywords.iter().filter(|kw| text.contains(kw.as_str())).count();
            let score = matches as f32 / keywords.len() as f32;
            ScoredPaper { paper, score, source }
        })
        .collect();
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    scored.truncate(10);
    scored
}

fn merge_and_rank(thunder: Vec<ScoredPaper>, phoenix: Vec<Paper>, topic: &str) -> Vec<ScoredPaper> {
    let in_network_ids: std::collections::HashSet<String> =
        thunder.iter().map(|sp| sp.paper.id.clone()).collect();
    let phoenix_scored = score_and_rank(
        phoenix.into_iter().filter(|p| !in_network_ids.contains(&p.id)).collect(),
        topic,
        "DISCOVERY",
    );
    let mut merged: Vec<ScoredPaper> = thunder.into_iter().chain(phoenix_scored).collect();
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    merged.truncate(15);
    merged
}

async fn fetch_paper_details(ids: &[String]) -> Vec<Paper> {
    if ids.is_empty() { return vec![]; }
    let url = format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
        ids.join(",")
    );
    let Ok(response) = reqwest::get(&url).await else { return vec![]; };
    let Ok(json) = response.json::<serde_json::Value>().await else { return vec![]; };
    let result = &json["result"];
    ids.iter()
        .filter_map(|id| {
            let entry = &result[id];
            if entry.is_null() { return None; }
            Some(Paper {
                id: id.clone(),
                title: entry["title"].as_str().unwrap_or("No title").to_string(),
                abstract_text: entry["summary"].as_str().unwrap_or("").to_string(),
                journal: entry["fulljournalname"].as_str().unwrap_or("Unknown journal").to_string(),
            })
        })
        .collect()
}
