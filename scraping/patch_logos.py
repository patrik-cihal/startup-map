"""Patch existing yc_company_details.csv to use small (square) logo URLs."""
import csv
import json
import requests
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading
import time
import re

input_csv = 'yc_company_details.csv'
output_csv = 'yc_company_details.csv'

progress_lock = threading.Lock()
progress_count = 0


def fetch_small_logo(row):
    global progress_count
    link = row['company_link']

    # Skip if already a small_logos URL
    if 'small_logos/' in row.get('logo_url', ''):
        with progress_lock:
            progress_count += 1
        return row

    headers = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'}
    try:
        r = requests.get(link, headers=headers, timeout=20)
        r.raise_for_status()
        # Extract JSON from the react component div
        m = re.search(r'ShowPage-react-component-[^"]*"[^>]*data-page="([^"]+)"', r.text)
        if m:
            import html
            data = json.loads(html.unescape(m.group(1)))
            company = data['props']['company']
            small_logo = company.get('small_logo_url', '')
            if small_logo:
                row['logo_url'] = small_logo
    except Exception as e:
        pass  # Keep existing logo_url on failure

    with progress_lock:
        progress_count += 1
        if progress_count % 50 == 0:
            print(f"  {progress_count}/{total} fetched...")

    return row


# Read existing CSV
with open(input_csv, 'r', encoding='utf-8') as f:
    reader = csv.DictReader(f)
    rows = list(reader)

total = len(rows)
need_patch = sum(1 for r in rows if 'small_logos/' not in r.get('logo_url', ''))
print(f"Loaded {total} companies, {need_patch} need logo patch")

start = time.time()
with ThreadPoolExecutor(max_workers=15) as executor:
    futures = [executor.submit(fetch_small_logo, row) for row in rows]
    results = [f.result() for f in futures]

elapsed = time.time() - start
print(f"Done in {elapsed:.0f}s")

# Write back
fieldnames = ['batch', 'company_link', 'name', 'tagline', 'long_description', 'founders', 'logo_url', 'location', 'founded', 'team_size', 'status']
with open(output_csv, 'w', newline='', encoding='utf-8') as f:
    writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction='ignore')
    writer.writeheader()
    writer.writerows(results)

patched = sum(1 for r in results if 'small_logos/' in r.get('logo_url', ''))
print(f"Result: {patched}/{total} companies now have square logos")
