#!/usr/bin/env python3

import netifaces
import re

def labNetwork():
    isLabNetwork = False
    for interface in netifaces.interfaces():
        config = netifaces.ifaddresses(interface)
        if netifaces.AF_INET in config.keys():
            for link in config[netifaces.AF_INET]:
                if 'addr' in link.keys() and 'peer' not in link.keys():
                    # print(link['addr'])
                    match = re.match('10.(0|1).24.(\d{1,3})', link['addr'])
                    if match is not None:
                        isLabNetwork = True
    if isLabNetwork:
        return True
