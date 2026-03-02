#!/usr/bin/env bash
set -euo pipefail

# ============================================
# Generate visual HTML benchmark report
# Usage: ./generate_report.sh <results.json>
# ============================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REPORT_DIR="$PROJECT_DIR/reports"

INPUT_FILE="${1:-$(ls -t "$REPORT_DIR"/benchmark_*.json 2>/dev/null | head -1)}"

if [ -z "$INPUT_FILE" ] || [ ! -f "$INPUT_FILE" ]; then
    echo "❌ No benchmark results found. Run ./scripts/benchmark.sh first."
    exit 1
fi

BASENAME=$(basename "$INPUT_FILE" .json)
HTML_FILE="$REPORT_DIR/${BASENAME}.html"

python3 -c "
import json, sys

with open('${INPUT_FILE}') as f:
    data = json.load(f)

t = data['tests']
hw = data['hardware']
mem = data['memory_mb']

# Build run detail rows
def run_rows(test_data):
    rows = ''
    for i, run in enumerate(test_data.get('runs', []), 1):
        rows += f'''<tr>
            <td>Run {i}</td>
            <td>{run['ttfb']:.3f}s</td>
            <td>{run['total']:.3f}s</td>
            <td>{run['tokens']}</td>
            <td>{run['tps']:.1f}</td>
        </tr>'''
    return rows

# Calculate overall best TPS
all_tps = [t[k]['tps_avg'] for k in ['short','medium','long'] if t[k]['tps_avg'] > 0]
best_tps = max(all_tps) if all_tps else 0

html = f'''<!DOCTYPE html>
<html lang=\"en\">
<head>
<meta charset=\"UTF-8\">
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
<title>LLM Benchmark Report</title>
<style>
  :root {{
    --bg: #0f172a;
    --surface: #1e293b;
    --surface2: #334155;
    --accent: #38bdf8;
    --accent2: #818cf8;
    --accent3: #34d399;
    --text: #f1f5f9;
    --text-dim: #94a3b8;
    --danger: #f87171;
    --warning: #fbbf24;
    --radius: 16px;
  }}
  * {{ margin: 0; padding: 0; box-sizing: border-box; }}
  body {{
    font-family: \"Inter\", -apple-system, BlinkMacSystemFont, sans-serif;
    background: var(--bg);
    color: var(--text);
    padding: 40px 20px;
    line-height: 1.6;
  }}
  .container {{ max-width: 900px; margin: 0 auto; }}
  
  /* Header */
  .header {{
    text-align: center;
    margin-bottom: 40px;
    padding: 40px;
    background: linear-gradient(135deg, rgba(56,189,248,0.1), rgba(129,140,248,0.1));
    border-radius: var(--radius);
    border: 1px solid rgba(56,189,248,0.2);
  }}
  .header h1 {{
    font-size: 2rem;
    background: linear-gradient(135deg, var(--accent), var(--accent2));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    margin-bottom: 8px;
  }}
  .header .meta {{ color: var(--text-dim); font-size: 0.9rem; }}
  
  /* Stat Cards */
  .stats {{ display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 32px; }}
  .stat {{
    background: var(--surface);
    border-radius: 12px;
    padding: 20px;
    text-align: center;
    border: 1px solid var(--surface2);
    transition: transform 0.2s;
  }}
  .stat:hover {{ transform: translateY(-2px); }}
  .stat .value {{
    font-size: 2rem;
    font-weight: 700;
    margin: 8px 0;
  }}
  .stat .label {{ color: var(--text-dim); font-size: 0.8rem; text-transform: uppercase; letter-spacing: 1px; }}
  .stat.blue .value {{ color: var(--accent); }}
  .stat.purple .value {{ color: var(--accent2); }}
  .stat.green .value {{ color: var(--accent3); }}
  .stat.yellow .value {{ color: var(--warning); }}
  
  /* Bar Chart */
  .chart-section {{
    background: var(--surface);
    border-radius: var(--radius);
    padding: 24px;
    margin-bottom: 24px;
    border: 1px solid var(--surface2);
  }}
  .chart-section h2 {{
    font-size: 1.2rem;
    margin-bottom: 20px;
    display: flex;
    align-items: center;
    gap: 8px;
  }}
  .bar-chart {{ display: flex; flex-direction: column; gap: 16px; }}
  .bar-row {{ display: flex; align-items: center; gap: 12px; }}
  .bar-label {{ width: 140px; font-size: 0.85rem; color: var(--text-dim); text-align: right; flex-shrink: 0; }}
  .bar-track {{ flex: 1; height: 32px; background: var(--bg); border-radius: 8px; overflow: hidden; position: relative; }}
  .bar-fill {{
    height: 100%;
    border-radius: 8px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 0.85rem;
    font-weight: 600;
    color: white;
    transition: width 0.6s ease;
    min-width: fit-content;
  }}
  .bar-fill.blue {{ background: linear-gradient(90deg, var(--accent), #0ea5e9); }}
  .bar-fill.purple {{ background: linear-gradient(90deg, var(--accent2), #6366f1); }}
  .bar-fill.green {{ background: linear-gradient(90deg, var(--accent3), #10b981); }}
  
  /* Table */
  table {{ width: 100%; border-collapse: collapse; margin-top: 16px; }}
  th {{ text-align: left; padding: 12px; color: var(--text-dim); font-size: 0.8rem; text-transform: uppercase; letter-spacing: 1px; border-bottom: 1px solid var(--surface2); }}
  td {{ padding: 12px; border-bottom: 1px solid rgba(51,65,85,0.5); font-variant-numeric: tabular-nums; }}
  tr:hover td {{ background: rgba(56,189,248,0.05); }}
  
  /* Hardware info */
  .hw-info {{
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    margin-top: 16px;
  }}
  .hw-item {{
    background: var(--bg);
    padding: 12px 16px;
    border-radius: 8px;
    font-size: 0.9rem;
  }}
  .hw-item .hw-label {{ color: var(--text-dim); font-size: 0.75rem; text-transform: uppercase; }}
  
  /* Footer */
  .footer {{
    text-align: center;
    margin-top: 40px;
    padding: 20px;
    color: var(--text-dim);
    font-size: 0.8rem;
  }}

  @media (max-width: 640px) {{
    .stats {{ grid-template-columns: repeat(2, 1fr); }}
    .hw-info {{ grid-template-columns: 1fr; }}
  }}
</style>
</head>
<body>
<div class=\"container\">
  <!-- Header -->
  <div class=\"header\">
    <h1>⚡ LLM Benchmark Report</h1>
    <div class=\"meta\">
      {data['model']}<br>
      {data['timestamp']} · {data['runs_per_test']} runs per test
    </div>
  </div>

  <!-- Stat Cards -->
  <div class=\"stats\">
    <div class=\"stat blue\">
      <div class=\"label\">Best TPS</div>
      <div class=\"value\">{best_tps:.1f}</div>
      <div class=\"label\">tokens/sec</div>
    </div>
    <div class=\"stat purple\">
      <div class=\"label\">Avg TTFT</div>
      <div class=\"value\">{t['short']['ttfb_avg']:.2f}s</div>
      <div class=\"label\">time to first token</div>
    </div>
    <div class=\"stat green\">
      <div class=\"label\">Memory</div>
      <div class=\"value\">{mem['after']}</div>
      <div class=\"label\">MB usage</div>
    </div>
    <div class=\"stat yellow\">
      <div class=\"label\">Runs</div>
      <div class=\"value\">{data['runs_per_test'] * 3}</div>
      <div class=\"label\">total tests</div>
    </div>
  </div>

  <!-- TPS Chart -->
  <div class=\"chart-section\">
    <h2>📊 Tokens per Second (TPS)</h2>
    <div class=\"bar-chart\">
      <div class=\"bar-row\">
        <div class=\"bar-label\">Short (20 tok)</div>
        <div class=\"bar-track\">
          <div class=\"bar-fill blue\" style=\"width: {min(t['short']['tps_avg']/max(best_tps,1)*100, 100):.0f}%\">{t['short']['tps_avg']:.1f} tok/s</div>
        </div>
      </div>
      <div class=\"bar-row\">
        <div class=\"bar-label\">Medium (200 tok)</div>
        <div class=\"bar-track\">
          <div class=\"bar-fill purple\" style=\"width: {min(t['medium']['tps_avg']/max(best_tps,1)*100, 100):.0f}%\">{t['medium']['tps_avg']:.1f} tok/s</div>
        </div>
      </div>
      <div class=\"bar-row\">
        <div class=\"bar-label\">Long (500 tok)</div>
        <div class=\"bar-track\">
          <div class=\"bar-fill green\" style=\"width: {min(t['long']['tps_avg']/max(best_tps,1)*100, 100):.0f}%\">{t['long']['tps_avg']:.1f} tok/s</div>
        </div>
      </div>
    </div>
  </div>

  <!-- TTFT Chart -->
  <div class=\"chart-section\">
    <h2>⏱️ Time to First Token (TTFT)</h2>
    <div class=\"bar-chart\">
      <div class=\"bar-row\">
        <div class=\"bar-label\">Short</div>
        <div class=\"bar-track\">
          <div class=\"bar-fill blue\" style=\"width: {min(t['short']['ttfb_avg']/max(t['long']['ttfb_avg'],0.001)*100, 100):.0f}%\">{t['short']['ttfb_avg']:.3f}s</div>
        </div>
      </div>
      <div class=\"bar-row\">
        <div class=\"bar-label\">Medium</div>
        <div class=\"bar-track\">
          <div class=\"bar-fill purple\" style=\"width: {min(t['medium']['ttfb_avg']/max(t['long']['ttfb_avg'],0.001)*100, 100):.0f}%\">{t['medium']['ttfb_avg']:.3f}s</div>
        </div>
      </div>
      <div class=\"bar-row\">
        <div class=\"bar-label\">Long</div>
        <div class=\"bar-track\">
          <div class=\"bar-fill green\" style=\"width: 100%\">{t['long']['ttfb_avg']:.3f}s</div>
        </div>
      </div>
    </div>
  </div>

  <!-- Detail Tables -->
  <div class=\"chart-section\">
    <h2>📋 Short Prompt — {t['short']['name']}</h2>
    <table>
      <tr><th>Run</th><th>TTFT</th><th>Total</th><th>Tokens</th><th>TPS</th></tr>
      {run_rows(t['short'])}
      <tr style=\"font-weight:700; border-top:2px solid var(--accent)\">
        <td>Average</td>
        <td>{t['short']['ttfb_avg']:.3f}s</td>
        <td>{t['short']['total_avg']:.3f}s</td>
        <td>{t['short']['tokens_avg']:.0f}</td>
        <td>{t['short']['tps_avg']:.1f}</td>
      </tr>
    </table>
  </div>

  <div class=\"chart-section\">
    <h2>📋 Medium Generation — {t['medium']['name']}</h2>
    <table>
      <tr><th>Run</th><th>TTFT</th><th>Total</th><th>Tokens</th><th>TPS</th></tr>
      {run_rows(t['medium'])}
      <tr style=\"font-weight:700; border-top:2px solid var(--accent2)\">
        <td>Average</td>
        <td>{t['medium']['ttfb_avg']:.3f}s</td>
        <td>{t['medium']['total_avg']:.3f}s</td>
        <td>{t['medium']['tokens_avg']:.0f}</td>
        <td>{t['medium']['tps_avg']:.1f}</td>
      </tr>
    </table>
  </div>

  <div class=\"chart-section\">
    <h2>📋 Long Generation — {t['long']['name']}</h2>
    <table>
      <tr><th>Run</th><th>TTFT</th><th>Total</th><th>Tokens</th><th>TPS</th></tr>
      {run_rows(t['long'])}
      <tr style=\"font-weight:700; border-top:2px solid var(--accent3)\">
        <td>Average</td>
        <td>{t['long']['ttfb_avg']:.3f}s</td>
        <td>{t['long']['total_avg']:.3f}s</td>
        <td>{t['long']['tokens_avg']:.0f}</td>
        <td>{t['long']['tps_avg']:.1f}</td>
      </tr>
    </table>
  </div>

  <!-- Hardware -->
  <div class=\"chart-section\">
    <h2>🖥️ Hardware</h2>
    <div class=\"hw-info\">
      <div class=\"hw-item\"><div class=\"hw-label\">Chip</div>{hw['chip']}</div>
      <div class=\"hw-item\"><div class=\"hw-label\">RAM</div>{hw['ram']}</div>
      <div class=\"hw-item\"><div class=\"hw-label\">Bandwidth</div>{hw['bandwidth']}</div>
    </div>
  </div>

  <div class=\"footer\">
    Generated by Mega LLM Server Benchmark Suite<br>
    Model: {data['model']}
  </div>
</div>
</body>
</html>'''

with open('${HTML_FILE}', 'w') as f:
    f.write(html)
"

echo ""
echo "✅ HTML Report saved: ${HTML_FILE}"
echo "   Open with: open \"${HTML_FILE}\""
