#!/usr/bin/env python3
"""Heimdall Benchmark Report Generator — Multi-model comparison HTML dashboard."""

import json
import sys

def generate_report(input_file: str, output_file: str):
    with open(input_file) as f:
        data = json.load(f)

    hw = data['hardware']
    mem = data['memory_mb']
    models = data.get('models', {})

    # Backward compat: old format had 'tests' at top level
    if not models and 'tests' in data:
        name = data.get('model', 'unknown')
        short = name.split('/')[-1] if '/' in name else name
        models = {name: {'name': name, 'short_name': short, 'tests': data['tests']}}

    model_list = list(models.values())
    num_models = len(model_list)
    is_multi = num_models > 1

    COLORS = [
        ('38,189,248', '#38bdf8'),
        ('129,140,248', '#818cf8'),
        ('52,211,153', '#34d399'),
        ('251,191,36', '#fbbf24'),
        ('248,113,113', '#f87171'),
        ('168,85,247', '#a855f7'),
    ]

    def color(idx):
        return COLORS[idx % len(COLORS)]

    all_tps = [m['tests'][k]['tps_avg'] for m in model_list for k in ['short', 'medium', 'long']]
    max_tps_val = max(all_tps) if all_tps else 1

    best_model = max(model_list, key=lambda m: max(m['tests'][k]['tps_avg'] for k in m['tests']))
    best_tps = max(best_model['tests'][k]['tps_avg'] for k in best_model['tests'])
    avg_ttft = sum(m['tests']['short']['ttfb_avg'] for m in model_list) / num_models

    # --- HTML builders ---

    def bars(test_key, title):
        rows = ''
        for i, m in enumerate(model_list):
            tps = m['tests'][test_key]['tps_avg']
            ttfb = m['tests'][test_key]['ttfb_avg']
            pct = min(tps / max(max_tps_val, 0.1) * 100, 100)
            rgb, hexc = color(i)
            rows += (
                f'<div class="bar-row">'
                f'<div class="bar-label">{m["short_name"][:25]}</div>'
                f'<div class="bar-track"><div class="bar-fill" '
                f'style="width:{pct:.0f}%;background:linear-gradient(90deg,rgba({rgb},0.9),rgba({rgb},0.6))">'
                f'{tps:.1f} tok/s</div></div>'
                f'<div class="bar-stat">{ttfb:.3f}s</div></div>\n'
            )
        return (
            f'<div class="chart-section"><h2>{title}</h2>'
            f'<div class="bar-header"><span></span><span>TPS →</span><span>TTFT</span></div>'
            f'<div class="bar-chart">{rows}</div></div>'
        )

    def detail(m, idx):
        _, hexc = color(idx)
        secs = ''
        for tk, tn in [('short', 'Short'), ('medium', 'Medium'), ('long', 'Long')]:
            t = m['tests'][tk]
            rows = ''
            for j, r in enumerate(t.get('runs', []), 1):
                rows += (
                    f'<tr><td>Run {j}</td><td>{r["ttfb"]:.3f}s</td>'
                    f'<td>{r["total"]:.3f}s</td><td>{r["tokens"]}</td>'
                    f'<td>{r["tps"]:.1f}</td></tr>'
                )
            rows += (
                f'<tr class="avg-row"><td>Avg</td><td>{t["ttfb_avg"]:.3f}s</td>'
                f'<td>{t["total_avg"]:.3f}s</td><td>{t["tokens_avg"]:.0f}</td>'
                f'<td>{t["tps_avg"]:.1f}</td></tr>'
            )
            secs += (
                f'<div class="detail-test"><h4>{tn} (max {t["max_tokens"]} tok)</h4>'
                f'<table><tr><th>Run</th><th>TTFT</th><th>Total</th><th>Tok</th><th>TPS</th></tr>'
                f'{rows}</table></div>'
            )
        return (
            f'<div class="model-detail" style="border-color:{hexc}">'
            f'<h3 style="color:{hexc}">📦 {m["short_name"]}</h3>'
            f'<div class="model-full-name">{m["name"]}</div>'
            f'<div class="detail-grid">{secs}</div></div>'
        )

    def summary_table():
        if not is_multi:
            return ''
        hdr = '<th>Metric</th>' + ''.join(
            f'<th style="color:{color(i)[1]}">{m["short_name"][:20]}</th>'
            for i, m in enumerate(model_list)
        )

        def row(label, fn):
            cells = ''.join(f'<td>{fn(m)}</td>' for m in model_list)
            return f'<tr><td>{label}</td>{cells}</tr>'

        rows = row('TTFT', lambda m: f'{m["tests"]["short"]["ttfb_avg"]:.3f}s')
        rows += row('TPS (short)', lambda m: f'{m["tests"]["short"]["tps_avg"]:.1f}')
        rows += row('TPS (medium)', lambda m: f'{m["tests"]["medium"]["tps_avg"]:.1f}')
        rows += row('TPS (long)', lambda m: f'{m["tests"]["long"]["tps_avg"]:.1f}')
        rows += row('Best TPS', lambda m: f'{max(m["tests"][k]["tps_avg"] for k in m["tests"]):.1f}')
        return f'<div class="chart-section"><h2>📊 Model Comparison</h2><table><tr>{hdr}</tr>{rows}</table></div>'

    winner = (
        f'<div class="winner"><span class="winner-icon">👑</span> '
        f'Best: <strong>{best_model["short_name"]}</strong> — {best_tps:.1f} tok/s</div>'
    ) if is_multi else ''

    version = data.get('version', '?')
    commit = data.get('git_commit', '?')

    CSS = '''
:root{--bg:#0f172a;--surface:#1e293b;--surface2:#334155;--text:#f1f5f9;--text-dim:#94a3b8;--radius:16px}
*{margin:0;padding:0;box-sizing:border-box}
body{font-family:"Inter",-apple-system,sans-serif;background:var(--bg);color:var(--text);padding:40px 20px;line-height:1.6}
.container{max-width:960px;margin:0 auto}
.header{text-align:center;margin-bottom:32px;padding:36px;background:linear-gradient(135deg,rgba(38,189,248,.1),rgba(129,140,248,.1));border-radius:var(--radius);border:1px solid rgba(38,189,248,.2)}
.header h1{font-size:2rem;background:linear-gradient(135deg,#38bdf8,#818cf8);-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:6px}
.header .meta{color:var(--text-dim);font-size:.9rem}
.header .version{display:inline-block;background:var(--surface2);padding:2px 10px;border-radius:20px;font-size:.8rem;margin-top:8px}
.stats{display:grid;grid-template-columns:repeat(4,1fr);gap:14px;margin-bottom:28px}
.stat{background:var(--surface);border-radius:12px;padding:18px;text-align:center;border:1px solid var(--surface2)}
.stat .value{font-size:1.8rem;font-weight:700;margin:6px 0}
.stat .label{color:var(--text-dim);font-size:.75rem;text-transform:uppercase;letter-spacing:1px}
.chart-section{background:var(--surface);border-radius:var(--radius);padding:22px;margin-bottom:20px;border:1px solid var(--surface2)}
.chart-section h2{font-size:1.1rem;margin-bottom:16px}
.bar-chart{display:flex;flex-direction:column;gap:10px}
.bar-header{display:flex;align-items:center;gap:12px;margin-bottom:4px;color:var(--text-dim);font-size:.75rem}
.bar-header span:first-child{width:140px;text-align:right;flex-shrink:0}
.bar-header span:nth-child(2){flex:1}
.bar-header span:last-child{width:60px;text-align:right}
.bar-row{display:flex;align-items:center;gap:12px}
.bar-label{width:140px;font-size:.8rem;color:var(--text-dim);text-align:right;flex-shrink:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.bar-track{flex:1;height:30px;background:var(--bg);border-radius:8px;overflow:hidden}
.bar-fill{height:100%;border-radius:8px;display:flex;align-items:center;padding:0 10px;font-size:.8rem;font-weight:600;color:#fff;min-width:fit-content}
.bar-stat{width:60px;text-align:right;font-size:.8rem;color:var(--text-dim)}
table{width:100%;border-collapse:collapse}
th{text-align:left;padding:10px;color:var(--text-dim);font-size:.75rem;text-transform:uppercase;letter-spacing:1px;border-bottom:1px solid var(--surface2)}
td{padding:10px;border-bottom:1px solid rgba(51,65,85,.4);font-variant-numeric:tabular-nums}
.avg-row td{font-weight:700;border-top:2px solid var(--surface2)}
.model-detail{border-radius:var(--radius);border:1px solid var(--surface2);border-left:4px solid;padding:20px;margin-bottom:20px;background:var(--surface)}
.model-detail h3{font-size:1.1rem;margin-bottom:4px}
.model-full-name{font-size:.8rem;color:var(--text-dim);margin-bottom:16px;font-family:monospace}
.detail-grid{display:grid;grid-template-columns:repeat(3,1fr);gap:16px}
.detail-test h4{font-size:.85rem;color:var(--text-dim);margin-bottom:8px}
.detail-test table{font-size:.85rem}
.winner{text-align:center;padding:14px;background:linear-gradient(135deg,rgba(251,191,36,.1),rgba(245,158,11,.1));border-radius:12px;margin-bottom:20px;border:1px solid rgba(251,191,36,.3);font-size:.95rem}
.winner-icon{font-size:1.3rem}
.hw-info{display:grid;grid-template-columns:repeat(3,1fr);gap:10px;margin-top:12px}
.hw-item{background:var(--bg);padding:10px 14px;border-radius:8px;font-size:.85rem}
.hw-item .hw-label{color:var(--text-dim);font-size:.7rem;text-transform:uppercase}
.footer{text-align:center;margin-top:32px;padding:16px;color:var(--text-dim);font-size:.8rem}
@media(max-width:700px){.stats{grid-template-columns:repeat(2,1fr)}.detail-grid{grid-template-columns:1fr}}
'''

    html = f'''<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1.0">
<title>Heimdall Benchmark Report</title>
<style>{CSS}</style></head>
<body><div class="container">
<div class="header">
<h1>🛡️ Heimdall Benchmark Report</h1>
<div class="meta">{data["timestamp"]} · {data["runs_per_test"]} runs per test · {num_models} model{"s" if is_multi else ""}</div>
<div class="version">v{version} ({commit})</div>
</div>

{winner}

<div class="stats">
<div class="stat"><div class="label">Best TPS</div><div class="value" style="color:#38bdf8">{best_tps:.1f}</div><div class="label">tokens/sec</div></div>
<div class="stat"><div class="label">Avg TTFT</div><div class="value" style="color:#818cf8">{avg_ttft:.2f}s</div><div class="label">time to first token</div></div>
<div class="stat"><div class="label">Memory</div><div class="value" style="color:#34d399">{mem["after"]}</div><div class="label">MB usage</div></div>
<div class="stat"><div class="label">Models</div><div class="value" style="color:#fbbf24">{num_models}</div><div class="label">benchmarked</div></div>
</div>

{summary_table()}
{bars("short", "⚡ Short Prompt — TPS Comparison")}
{bars("medium", "📝 Medium Generation — TPS Comparison")}
{bars("long", "📖 Long Generation — TPS Comparison")}

<h2 style="margin:28px 0 16px;font-size:1.2rem">📋 Detailed Results per Model</h2>
{"".join(detail(m, i) for i, m in enumerate(model_list))}

<div class="chart-section"><h2>🖥️ Hardware</h2>
<div class="hw-info">
<div class="hw-item"><div class="hw-label">Chip</div>{hw["chip"]}</div>
<div class="hw-item"><div class="hw-label">RAM</div>{hw["ram"]}</div>
<div class="hw-item"><div class="hw-label">Bandwidth</div>{hw["bandwidth"]}</div>
</div></div>

<div class="footer">Heimdall Benchmark Suite v{version}</div>
</div></body></html>'''

    with open(output_file, 'w') as f:
        f.write(html)


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: report_template.py <input.json> <output.html>")
        sys.exit(1)
    generate_report(sys.argv[1], sys.argv[2])
