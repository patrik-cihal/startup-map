import os
from selenium import webdriver
from selenium.webdriver.chrome.service import Service
from selenium.webdriver.chrome.options import Options
from webdriver_manager.chrome import ChromeDriverManager
from webdriver_manager.core.os_manager import ChromeType
from bs4 import BeautifulSoup
import time
import csv
from urllib.parse import quote

# List of batches as provided by the user
batches = [
    "Summer 2025",
    "Spring 2025",
    "Winter 2025",
    "Fall 2024",
    "Summer 2024",
    "Winter 2024",
    "Summer 2023",
    "Winter 2023",
    "Summer 2022",
    "Winter 2022",
    "Summer 2021",
    "Winter 2021",
    "Summer 2020",
    "Winter 2020",
    "Summer 2019",
    "Winter 2019",
    "Summer 2018",
    "Winter 2018",
    "Summer 2017",
    "Winter 2017",
    "Summer 2016",
    "Winter 2016",
    "Summer 2015",
    "Winter 2015",
    "Summer 2014",
    "Winter 2014",
    "Summer 2013",
    "Winter 2013",
    "Summer 2012",
    "Winter 2012",
    "Summer 2011",
    "Winter 2011",
    "Summer 2010",
    "Winter 2010",
    "Summer 2009",
    "Winter 2009",
    "Summer 2008",
    "Winter 2008",
    "Summer 2007",
    "Winter 2007",
    "Summer 2006",
    "Winter 2006",
    "Summer 2005"
]

# CSV file name
csv_file = 'yc_company_links.csv'

# Setup Selenium with Chrome/Chromium
options = Options()
options.headless = True  # Run in headless mode
options.binary_location = "/usr/sbin/chromium"  # Path to your Chromium binary (adjust if needed)

# Specify ChromeDriver for Chromium
try:
    driver = webdriver.Chrome(
        service=Service(ChromeDriverManager(chrome_type=ChromeType.CHROMIUM).install()),
        options=options
    )
except Exception as e:
    print(f"Error initializing ChromeDriver: {e}")
    print("Ensure Chromium is installed and the binary path is correct.")
    exit(1)

# Check if CSV exists; if not, create it with header
header = ['batch', 'company_link']
if not os.path.exists(csv_file):
    with open(csv_file, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=header)
        writer.writeheader()

# Total count
total_companies = 0

for batch in batches:
    # Construct URL with encoded batch name
    encoded_batch = quote(batch)
    url = f"https://www.ycombinator.com/companies?batch={encoded_batch}"

    try:
        # Load the page
        driver.get(url)

        # Scroll to load all companies (lazy loading)
        last_height = driver.execute_script("return document.body.scrollHeight")
        while True:
            driver.execute_script("window.scrollTo(0, document.body.scrollHeight);")
            time.sleep(3)  # Increased wait time for reliability
            new_height = driver.execute_script("return document.body.scrollHeight")
            if new_height == last_height:
                break
            last_height = new_height

        # Parse the loaded page source with BeautifulSoup
        soup = BeautifulSoup(driver.page_source, 'html.parser')

        # Find all company cards (adjust selector if needed)
        company_cards = soup.find_all('a', class_=lambda x: x and '_company_' in x)

        if not company_cards:
            print(f"No companies found for batch {batch}. Check URL or selectors.")
            continue

        # List for this batch's links
        batch_links = []
        for card in company_cards:
            href = card.get('href')
            if href and href.startswith('/companies/'):
                link = f"https://www.ycombinator.com{href}"
                batch_links.append({
                    'batch': batch,
                    'company_link': link
                })

        # Append to CSV
        with open(csv_file, 'a', newline='', encoding='utf-8') as f:
            writer = csv.DictWriter(f, fieldnames=header)
            writer.writerows(batch_links)

        num_scraped = len(batch_links)
        total_companies += num_scraped
        print(f"Scraped {num_scraped} company links from batch {batch} and appended to {csv_file}")

    except Exception as e:
        print(f"Error processing batch {batch}: {e}")
        continue

# Close the driver
driver.quit()

print(f"Total scraped {total_companies} company links across all batches.")
