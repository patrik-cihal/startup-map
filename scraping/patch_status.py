"""Patch existing yc_company_details.csv with ycdc_status from YC website."""
import csv
import json
import requests
from bs4 import BeautifulSoup
from concurrent.futures import ThreadPoolExecutor, as_completed
import threading
import time

input_csv = 'yc_company_details.csv'
output_csv = 'yc_company_details.csv'

progress_lock = threading.Lock()
progress_count = 0

def fetch_status(row):
    global progress_count
    link = row['company_link']
    # Skip if already has status
    if row.get('status') and row['status'] not in ('', 'N/A'):
        with progress_lock:
            progress_count += 1
        return row

    headers = {'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36'}
    try:
        r = requests.get(link, headers=headers, timeout=20)
        r.raise_for_status()
        soup = BeautifulSoup(r.text, 'html.parser')
        div = soup.find('div', id=lambda x: x and 'ShowPage-react-component' in str(x))
        if div:
            data = json.loads(div['data-page'])
            company = data['props']['company']
            row['status'] = company.get('ycdc_status', 'Active')
        else:
            row['status'] = 'Active'
    except Exception as e:
        row['status'] = 'Active'

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
need_status = sum(1 for r in rows if not r.get('status') or r['status'] in ('', 'N/A'))
print(f"Loaded {total} companies, {need_status} need status fetch")

start = time.time()
with ThreadPoolExecutor(max_workers=15) as executor:
    futures = [executor.submit(fetch_status, row) for row in rows]
    results = [f.result() for f in futures]

elapsed = time.time() - start
print(f"Done in {elapsed:.0f}s")

# Write back
fieldnames = ['batch', 'company_link', 'name', 'tagline', 'long_description', 'founders', 'logo_url', 'location', 'founded', 'team_size', 'status']
with open(output_csv, 'w', newline='', encoding='utf-8') as f:
    writer = csv.DictWriter(f, fieldnames=fieldnames, extrasaction='ignore')
    writer.writeheader()
    writer.writerows(results)

# Stats
statuses = {}
for r in results:
    s = r.get('status', 'Unknown')
    statuses[s] = statuses.get(s, 0) + 1
print("Status distribution:")
for s, c in sorted(statuses.items(), key=lambda x: -x[1]):
    print(f"  {s}: {c}")
