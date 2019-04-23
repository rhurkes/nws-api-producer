#!/usr/bin/env python3

import json
import requests
import time

LSRS_URL = 'https://api.weather.gov/products/types/lsr'

# Get list of LSRs
text = requests.get(LSRS_URL).text
data = json.loads(text)
urls = [x['@id'] for x in data['@graph']]

# Write
with open('lsrs.txt', 'a') as dest:
    dest.truncate(0)
    for url in urls:
        text = requests.get(url).text
        if "STORM REPORT...SUMMARY" not in text:
            data = json.loads(text)
            dest.write(url + '\n')
            dest.write(data['productText'] + '\n========================================\n')
        time.sleep(1)
