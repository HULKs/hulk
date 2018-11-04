#!/usr/bin/env python
# -*- coding: utf-8 -*-
import os
import json

def getNaoIdList(location = "default"):
    '''
    Read id_map.json and extract nao Id list.
    @param location, default value is "default"
    @return array of strings
    '''
    script_dir = os.path.dirname(os.path.realpath(__file__))
    REL_CONF_LOCATION = os.path.join('..', 'home', 'configuration', 'location')
    FILE_NAME = 'id_map.json'
    CONF_LOCATION = os.path.abspath(os.path.join(script_dir, REL_CONF_LOCATION))
    try:
        file_path = os.path.join(CONF_LOCATION, location, FILE_NAME)
        if not os.path.isfile(file_path):
            if location == 'default':
                raise ValueError() # this will divert to the except handling
            else:
                print("Could not find id_map.json at location \"" + location + "\", Fallback to \"default\"")
                location = 'default'
                file_path = os.path.join(CONF_LOCATION, location, FILE_NAME)
        with open(file_path) as f:
            return json.load(f).get("idmap.nao", [])
    except:
        print("ERROR," + FILE_NAME + " not found on default location!!!")

def getNaoNames(location = "default"):
    '''
    Return nao names as an array of strings. ie: [tuhhnao01]
    @param location, default value is "default"
    @return array of strings
    '''
    data = getNaoIdList(location)
    return list(map(lambda n: str(n["name"]), data))

def getNaoNumbers(location = "default"):
    '''
    Return nao numbers as an array of ints.
    @param location, default value is "default"
    @return array of ints
    '''
    names = getNaoNames(location)
    NAO_NAME_PREFIX_LEN = len('tuhhnao')
    return list(map(lambda n: int(n[NAO_NAME_PREFIX_LEN:]), names))

if __name__ == '__main__':
    '''
    Standalone mode. Probably useful to get available nao's for a given location
    '''

    import argparse
    parser = argparse.ArgumentParser(description='Return Nao names or numbers.')
    parser.add_argument('--location', dest='location', default='default',
                    help='Location, ie: smd. Default = "default".')
    parser.add_argument('--names', action='store_true', help = "if set, return Nao names else return Nao Numbers.")
    args = parser.parse_args()

    if args.names:
        print(getNaoNames(args.location))
    else:
        print(getNaoNumbers(args.location))
