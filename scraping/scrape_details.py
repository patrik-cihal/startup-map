import csv
import json
import requests
from bs4 import BeautifulSoup
from concurrent.futures import ThreadPoolExecutor, as_completed
from urllib.parse import urljoin
import os
import threading

# Input and output files
input_csv = 'yc_company_links.csv'
output_csv = 'yc_company_details.csv'

# Thread-safe counter for progress tracking
progress_lock = threading.Lock()
progress_count = 0
total_links = 0

# Function to scrape a single company page
def scrape_company(batch, link):
    global progress_count  # Declare global at the start
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36'
    }
    try:
        response = requests.get(link, headers=headers, timeout=20)
        response.raise_for_status()
        soup = BeautifulSoup(response.text, 'html.parser')

        # Find the div with id containing 'ycdc_new/pages/Companies/ShowPage-react-component-'
        react_div = soup.find('div', id=lambda x: x and 'ycdc_new/pages/Companies/ShowPage-react-component-' in x)
        if not react_div:
            raise ValueError("React component div not found")

        # Get the data-page attribute and parse as JSON
        data_page = react_div.get('data-page')
        if not data_page:
            raise ValueError("data-page attribute not found")

        page_data = json.loads(data_page)
        company = page_data['props']['company']

        # Extract details from JSON
        name = company.get('name', 'N/A')
        tagline = company.get('one_liner', 'N/A')
        long_description = company.get('long_description', 'N/A')

        founders = []
        for founder in company.get('founders', []):
            founders.append(founder.get('full_name', 'N/A'))
        founders_str = ', '.join(founders) if founders else 'N/A'

        logo_url = company.get('logo_url', 'N/A')
        location = company.get('location', 'N/A')
        founded = company.get('year_founded', 'N/A')
        team_size = company.get('team_size', 'N/A')

        # Update progress and print lengths
        with progress_lock:
            progress_count += 1
            print(f"Processed {progress_count}/{total_links}: {batch} - {link}")
            print(f"Tagline Length: {len(tagline)}")
            print(f"Long Description Length: {len(long_description)}")
            print("---")

        return {
            'batch': batch,
            'company_link': link,
            'name': name,
            'tagline': tagline,
            'long_description': long_description,
            'founders': founders_str,
            'logo_url': logo_url,
            'location': location,
            'founded': founded,
            'team_size': team_size
        }
    except Exception as e:
        with progress_lock:
            progress_count += 1
            print(f"Error {progress_count}/{total_links}: {batch} - {link} - {e}")
        return None

# Read links from CSV
companies = []
try:
    with open(input_csv, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        companies = list(reader)
except FileNotFoundError:
    print(f"Error: {input_csv} not found. Ensure the file exists.")
    exit(1)

# Load existing details to skip already-scraped companies
existing_results = []
existing_links = set()
if os.path.exists(output_csv):
    with open(output_csv, 'r', encoding='utf-8') as f:
        reader = csv.DictReader(f)
        existing_results = list(reader)
        existing_links = {row['company_link'] for row in existing_results}

new_companies = [c for c in companies if c['company_link'] not in existing_links]
total_links = len(new_companies)
print(f"Found {len(existing_results)} existing details, {total_links} new companies to scrape.")

# Scrape new companies in parallel
new_results = []
if new_companies:
    with ThreadPoolExecutor(max_workers=10) as executor:
        futures = [executor.submit(scrape_company, company['batch'], company['company_link']) for company in new_companies]
        for future in as_completed(futures):
            result = future.result()
            if result:
                new_results.append(result)

# Merge existing + new and save
all_results = existing_results + new_results
if all_results:
    fieldnames = ['batch', 'company_link', 'name', 'tagline', 'long_description', 'founders', 'logo_url', 'location', 'founded', 'team_size']
    try:
        with open(output_csv, 'w', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()
            writer.writerows(all_results)
        print(f"Saved {len(all_results)} total ({len(new_results)} new) company details to {output_csv}")
    except Exception as e:
        print(f"Error saving to {output_csv}: {e}")
else:
    print("No data scraped.")
