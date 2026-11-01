# RustXNG

[![CI](https://github.com/sahenjp/RustXNG/actions/workflows/ci.yml/badge.svg)](https://github.com/sahenjp/RustXNG/actions/workflows/ci.yml)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange.svg)](Cargo.toml)
[![Release](https://img.shields.io/github/v/release/sahenjp/RustXNG?sort=semver)](https://github.com/sahenjp/RustXNG/releases/latest)

RustXNG is a self-hosted metasearch engine, a Rust rewrite of [SearXNG](https://github.com/searxng/searxng) (which is written in Python). Queries are sent directly to more than 200 search sites, including Google, Bing, Wikipedia and GitHub, and results are shown without ads or tracking. All 226 engines that SearXNG provides by default have been ported.

The distribution is a single binary: no Python runtime, no database, just one settings file. It starts in 0.03 seconds and settles at 13 MiB of memory.

## Features

- **Metasearch**: one query across 200+ sites, covering general web, news, images, video, music, books, academic papers, package registries, maps and more
- **Merged results**: duplicates removed and results ranked by relevance; when some sites are down, the rest still answer
- **Multiple output formats**: a browser UI plus JSON, CSV and RSS
- **Privacy**: images and favicons are served through a proxy, so your IP address is never exposed to the target sites
- **Operations features**: search suggestions, preferences UI, statistics page, health check, metrics output, shared Valkey / Redis cache, and rate limiting against bots
- **Search your own data**: PostgreSQL / MySQL / MariaDB / MongoDB / SQLite / Valkey / Elasticsearch / Solr / Meilisearch can be included as search targets
- **Local index (experimental, off by default)**: pages you crawl yourself are indexed with BM25 and merged into upstream results; active only when `local_index_path` is set
- **Usable directly from rdsh**: the default port 8888 matches rdsh's probe target, so `rdsh search-web --engine rustxng` works as is

## Usage

### Get the binary

Prebuilt binaries are published on the releases page, named with the release tag and the Rust target:

| Platform | Example asset name | Notes |
| :--- | :--- | :--- |
| Linux x86_64 (glibc) | `rustxng-v0.4.0-x86_64-unknown-linux-gnu` | glibc 2.34 or newer |
| Linux x86_64 (static) | `rustxng-v0.4.0-x86_64-unknown-linux-musl` | runs on Alpine and the like |
| macOS arm64 | `rustxng-v0.4.0-aarch64-apple-darwin` | Apple Silicon |
| macOS x86_64 | `rustxng-v0.4.0-x86_64-apple-darwin` | Intel Macs |
| Windows x86_64 | `rustxng-v0.4.0-x86_64-pc-windows-msvc.exe` | save it as `rustxng.exe` |

Every asset has a `.sha256` checksum next to it. Verify the download, then put the binary in place:

```bash
gh release download --repo sahenjp/RustXNG --pattern 'rustxng-*-x86_64-unknown-linux-gnu*'
sha256sum -c rustxng-v0.4.0-x86_64-unknown-linux-gnu.sha256
install -m755 rustxng-v0.4.0-x86_64-unknown-linux-gnu ~/.local/bin/rustxng
```

`rdsh rustxng install` (bundled with rustdsh) follows this naming scheme, picks the latest release asset and installs it automatically.

### Run it

```bash
cargo run --release -- --port 8888
```

Open <http://127.0.0.1:8888/> to search. With Docker:

```bash
docker build -t rustxng .
docker run --rm -p 8888:8888 rustxng
```

### Use it from a program

```bash
# Get results as JSON
curl 'http://127.0.0.1:8888/search?q=rust&format=json'

# Only pages published within the last day
curl 'http://127.0.0.1:8888/search?q=rust&time_range=day&format=json'
```

Prefix the query with `!sitename` to search a single site:

```bash
curl 'http://127.0.0.1:8888/search?q=%21wikipedia+rust&format=json'   # Wikipedia only
curl 'http://127.0.0.1:8888/search?q=%21github+tokio&format=json'      # GitHub only
```

### Parameters for AI agents

For LLM tool calls, you can specify how many results come back and whether page content is included. These parameters do not exist in SearXNG; leaving them out keeps the previous behavior.

| Parameter | Meaning | Measured (`q=rust async`) |
| :--- | :--- | ---: |
| `max_results=5` | return only the top N results (1–200) | 60 KB → **4.6 KB** (-92%) |
| `include_domains=github.com` | keep only this domain and its subdomains | — |
| `exclude_domains=example.com` | drop this domain | — |
| `include_raw_content=1` | fetch page content for the top 5 results into `raw_content` | +16 KB / +3.5 s |
| `strategy=staged` | start with a few engines and stop once enough results are in (default is `parallel`) | 3.0 s → **0.84 s** (123 → 55 results) |

**Recommended usage, compared with commercial APIs** (measured with the default `settings.toml`):

- **On par with or better than Exa** (100 questions, no API key): with the local index merged in, hit@1 **83** vs 77, hit@5 **96** vs 91, MRR **0.889** vs 0.840. Without the index it still leads at 79 / 92 / 0.853
- **Ahead of Tavily by a wide margin**: hit@1 81 vs 34, hit@5 93 vs 79, MRR 0.870 vs 0.514, at $0 vs $8 per 1k queries
- **Raw speed alone trails Exa** (0.8–1.6 s vs 0.60 s; repeating the same query takes 0.01 s) because queries fan out to 180 engines in parallel. Details in [docs/BENCHMARK.md](docs/BENCHMARK.md)
- **Google is not usable** (measured): the WML edition returns 403, the regular one requires JavaScript, and headless browsers are rejected as bots. The figures above exclude Google
- **Plain-language explanation**: [docs/COMPARISON-EXA.md](docs/COMPARISON-EXA.md) compares Exa for non-technical readers
- **SearXNG compatibility is preserved**: single-engine queries keep upstream ordering (overlap@10 = 1.00); `relevance_weight = 0` / `authority_weight = 0` restores the exact upstream order

For agents, this one call is recommended:

```bash
curl 'http://127.0.0.1:8888/search?q=<query>&format=json&max_results=5&include_raw_content=1'
```

`include_raw_content=1` attaches page content, and fetched pages are cached, so every call after the first returns immediately. Details in [docs/BENCHMARK.md](docs/BENCHMARK.md).

### Deep research

For complex questions, a single endpoint plans sub-queries, searches several times, collects page content, and returns a cited markdown report when an LLM is configured.

```bash
# Evidence mode (no LLM required): returns sub-queries, sources and page content, structured
curl 'http://127.0.0.1:8888/research?q=<question>'

# Report mode: with an LLM configured in the [research] section of settings.toml, adds a cited report
curl 'http://127.0.0.1:8888/research?q=<question>&max_queries=5&max_sources=10'
```

To use an LLM, set an OpenAI-compatible `base_url` and `api_key` in the `[research]` section (without one, the evidence bundle is still returned).

## Supported engines

All **226 engines** that SearXNG provides by default have been ported (352 settings entries). Engines that upstream does not have, such as **Yahoo! JAPAN** for Japanese search, have been added on top.

| Category | Examples |
| :--- | :--- |
| Web search | Google, Bing, DuckDuckGo, Brave, Yahoo, Yandex, Qwant, Startpage |
| News | Google News, Bing News, Reuters, Yahoo News, regional news publishers |
| Images, video, music | Google Images, YouTube, Vimeo, Bilibili, Spotify, SoundCloud |
| Academic papers | arXiv, PubMed, Crossref, OpenAlex, Semantic Scholar, Springer |
| Development, packages | GitHub, GitLab, npm, PyPI, crates.io, Docker Hub, Hex, Go |
| Encyclopedias, dictionaries | Wikipedia, Wikidata, Wiktionary, various dictionaries and translators |
| Other | weather, currency conversion, maps, torrents, file sharing and more |

About 170 sites are enabled by default. Sites that need an API key or your own server are left disabled; enable them in [settings.toml](settings.toml). The full list is in [docs/COVERAGE.md](docs/COVERAGE.md).

## Performance

Measured against SearXNG (the Python version) on the same machine. Conditions and reproduction steps are in [docs/BENCHMARK.md](docs/BENCHMARK.md).

| Metric | SearXNG | RustXNG |
| :--- | ---: | ---: |
| Startup time | 0.72 s | **0.03 s** |
| Memory usage | 118 MiB | **13 MiB** |
| Search across all default engines (175) | 1–3 s | **0.27 s** (warm, median) |
| Single-site search | 100–400 ms | **1–50 ms** |
| Result agreement | — | exact match on 6 of the 8 main engines (overlap@10 = 1.00) |

To measure in your own environment:

```bash
cargo run --release --example benchmark
```

## Configuration

The bundled [settings.toml](settings.toml) holds the defaults. Pass a TOML file containing only the keys you want to change via `--config`; it is merged over the defaults.

```toml
[server]
port = 8888
image_proxy = true

[search]
safe_search = 1
default_lang = "ja"
```

If you are coming from SearXNG, you can pass your existing `settings.yml` (YAML) directly to `--config`. Environment variables keep the SearXNG names (`SEARXNG_PORT` and the like). Details in [docs/PORTING.md](docs/PORTING.md).

### Automatic engine management

Only some of the 169 engines are usable at any given time. Two mechanisms sort this out automatically:

| Mechanism | Behavior | Settings |
| --- | --- | --- |
| Suspension | engines that keep failing (timeouts, 429s, CAPTCHAs, refusals) are skipped for a set period | `ban_time_on_fail` / `suspended_times` |
| Dormancy | engines that keep succeeding without returning any results are skipped for a set period | `empty_result_limit` (default 8) / `empty_result_cooldown` (default 300 s) |

Dormancy targets engines that are not failing but not contributing either: it prevents engines blocked into returning empty pages from burning the timeout on every query. Returning even a single result restores the engine immediately.

### Proxies

Useful for bringing blocked engines (Google, Bing and others) back:

```toml
[outgoing.proxies]
https = "socks5h://127.0.0.1:9050"
http = "socks5h://127.0.0.1:9050"
```

### Per-engine proxies

Only the blocked engines can be routed through a proxy. This is the counterpart of upstream SearXNG's per-engine `proxies`.

```toml
[[engines]]
name = "google"
engine = "google"
proxy = "http://127.0.0.1:8080"      # this engine only
```

Engines without an explicit `proxy` keep direct access (with `outgoing.proxies` set, it applies to all engines).

**Verified in practice**: with a local test proxy, only the engine configured with `proxy` issued `CONNECT <host>:443`, and no other engine was affected.

### Running as a sidecar

Set `RUSTXNG_IDLE_TIMEOUT=300` and the process exits on its own after 5 minutes without requests. This prevents resident processes started by launchers from lingering.

## Differences from SearXNG

Search syntax, parameter names, API responses and the configuration model follow SearXNG. What differs:

- The UI design is its own (SearXNG's themes have not been ported)
- The UI is translated into 57 languages (SearXNG has 60+). The search language can be chosen from 110+ options
- Plugins: 11 of SearXNG's 12

Compatibility details are in [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets
cargo test          # 735 tests, no external network required
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to add engines and [SECURITY.md](SECURITY.md) for reporting vulnerabilities. CI runs formatting, clippy and tests on Linux, macOS and Windows, and also verifies a `--no-default-features` lightweight build and Docker image startup.

## Documentation

| Document | Contents |
| :--- | :--- |
| [docs/INTRO.md](docs/INTRO.md) | **Introduction for wiring it into AI agents** (use cases, integration examples) |
| [docs/ACCURACY.md](docs/ACCURACY.md) | Measured result agreement with SearXNG (overlap@10) |
| [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) | What matches SearXNG and what differs |
| [docs/COVERAGE.md](docs/COVERAGE.md) | The list of supported search sites |
| [docs/PORTING.md](docs/PORTING.md) | Internals and configuration in depth |
| [docs/BENCHMARK.md](docs/BENCHMARK.md) | How speed is measured, and the numbers |
| [docs/LOCAL-INDEX.md](docs/LOCAL-INDEX.md) | Local index (experimental: crawling and BM25 merging) |
| [docs/RDSH.md](docs/RDSH.md) | Using it from rdsh |
| [docs/COMPARISON-EXA.md](docs/COMPARISON-EXA.md) | The Exa comparison, explained for non-technical readers |
| [SECURITY.md](SECURITY.md) | Vulnerability scan results and known caveats |

## License

**AGPL-3.0-or-later** ([LICENSE](LICENSE)). This is a derivative work based on SearXNG and other upstream projects; attribution is listed in [NOTICE](NOTICE). Under section 13 of the AGPL, if you offer a modified version as a network service, you must provide the corresponding source code.
