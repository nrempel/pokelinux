#!/usr/bin/env python3
"""Issue one offline pairing bundle from an Android-generated public-key request.

The phone's private key never leaves AndroidKeyStore. This command creates local
client/CA keys only when explicitly run. It never contacts a phone or network.
"""
import argparse
import base64
import hashlib
import json
import os
import re
import subprocess
import tempfile
import zipfile
from pathlib import Path
from omarchy_bridge import _object


def _openssl(*args, input=None):
    try:
        return subprocess.run(['openssl',*map(str,args)],input=input,capture_output=True,check=True,timeout=30).stdout
    except subprocess.CalledProcessError as error:
        raise ValueError('OpenSSL rejected enrollment material') from error


def issue_bundle(request_file, destination, days=90):
    if type(days) is not int or not 1<=days<=90:
        raise ValueError('certificate lifetime must be 1 to 90 days')
    request_file=Path(request_file); destination=Path(destination).absolute()
    with request_file.open('rb') as source:
        raw=source.read(8193)
    if len(raw)>8192: raise ValueError('request exceeds size limit')
    request=json.loads(raw,object_pairs_hook=_object)
    if not isinstance(request,dict) or set(request)!={'v','request_id','public_key'} or type(request['v']) is not int or request['v']!=1:
        raise ValueError('invalid enrollment request')
    if not isinstance(request['request_id'],str) or not re.fullmatch('[0-9a-f]{32}',request['request_id']):
        raise ValueError('invalid enrollment request id')
    try: public=base64.b64decode(request['public_key'],validate=True)
    except (ValueError,TypeError) as error: raise ValueError('invalid public key') from error
    if not 32<=len(public)<=2048: raise ValueError('invalid public key length')
    public_pem=_openssl('pkey','-pubin','-inform','DER','-pubout',input=public)
    # Exclusive creation prevents accidental replacement or rotation of any identity.
    destination.mkdir(mode=0o700)
    old_mask=os.umask(0o077)
    try:
        (destination/'phone-public.pem').write_bytes(public_pem)
        for name in ('ca','client'):
            _openssl('genpkey','-algorithm','EC','-pkeyopt','ec_paramgen_curve:P-256','-out',destination/f'{name}.key')
        _openssl('req','-new','-x509','-key',destination/'ca.key','-out',destination/'ca.pem','-days',days,
                 '-subj','/CN=Omarchy local pairing CA','-addext','basicConstraints=critical,CA:TRUE,pathlen:0',
                 '-addext','keyUsage=critical,keyCertSign,cRLSign')
        for name, purpose in (('server','serverAuth'),('client','clientAuth')):
            ext=destination/f'{name}.ext'
            ext.write_text('basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage='+purpose+'\n'+('subjectAltName=DNS:omarchy-android\n' if name=='server' else ''))
            common=['x509','-new','-CA',destination/'ca.pem','-CAkey',destination/'ca.key','-set_serial','0x'+os.urandom(16).hex(),
                    '-days',days,'-subj','/CN='+('omarchy-android' if name=='server' else 'Omarchy desktop'),'-extfile',ext,'-out',destination/f'{name}.pem']
            if name=='server': common+=['-force_pubkey',destination/'phone-public.pem']
            else:
                (destination/'client-public.pem').write_bytes(_openssl('pkey','-in',destination/'client.key','-pubout'))
                common+=['-force_pubkey',destination/'client-public.pem']
            _openssl(*common)
        ders={name:_openssl('x509','-in',destination/f'{name}.pem','-outform','DER') for name in ('server','client','ca')}
        pins={name+'_sha256':hashlib.sha256(value).hexdigest() for name,value in ders.items()}
        with zipfile.ZipFile(destination/'phone-enrollment.zip','w',compression=zipfile.ZIP_STORED) as bundle:
            bundle.writestr('request-id',request['request_id'])
            for name,value in ders.items(): bundle.writestr(name+'.der',value)
        config={'host':'127.0.0.1','port':37179,'server_name':'omarchy-android','server_sha256':pins['server_sha256'],
                'ca':str(destination/'ca.pem'),'certificate':str(destination/'client.pem'),'private_key':str(destination/'client.key')}
        (destination/'client.json').write_text(json.dumps(config,indent=2)+'\n')
        pins['bundle_sha256']=hashlib.sha256((destination/'phone-enrollment.zip').read_bytes()).hexdigest()
        pins['request_sha256']=hashlib.sha256(raw).hexdigest()
        (destination/'fingerprints.json').write_text(json.dumps(pins,indent=2)+'\n')
        return pins
    finally:
        os.umask(old_mask)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('request',type=Path)
    parser.add_argument('destination',type=Path)
    parser.add_argument('--days',type=int,default=90)
    args=parser.parse_args()
    print(json.dumps(issue_bundle(args.request,args.destination,args.days),indent=2))

if __name__=='__main__': main()
