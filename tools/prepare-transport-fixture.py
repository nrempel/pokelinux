#!/usr/bin/env python3
"""Create short-lived disposable emulator certificates. Never deployment identities."""
import hashlib,json,os,subprocess
from pathlib import Path
root=Path('android-framework/transport-fixture').resolve();root.mkdir(parents=True,exist_ok=True);os.chmod(root,0o700)
def run(*args):subprocess.run(args,cwd=root,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
run('openssl','req','-x509','-newkey','rsa:2048','-nodes','-keyout','ca.key','-out','ca.pem','-subj','/CN=Disposable emulator CA','-days','2','-addext','basicConstraints=critical,CA:TRUE','-addext','keyUsage=critical,keyCertSign,cRLSign')
for name,purpose in [('server','serverAuth'),('client','clientAuth'),('other','clientAuth'),('expired','clientAuth'),('wrongpurpose','serverAuth'),('untrusted','clientAuth')]:
 run('openssl','req','-new','-newkey','rsa:2048','-nodes','-keyout',name+'.key','-out',name+'.csr','-subj','/CN=omarchy-'+name)
 (root/(name+'.ext')).write_text('basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage='+purpose+'\nsubjectAltName=DNS:omarchy-android\n')
 if name=='untrusted':
  run('openssl','x509','-req','-in',name+'.csr','-signkey',name+'.key','-out',name+'.pem','-days','2','-extfile',name+'.ext')
 else:
  run('openssl','x509','-req','-in',name+'.csr','-CA','ca.pem','-CAkey','ca.key','-CAcreateserial','-out',name+'.pem','-days','0' if name=='expired' else '2','-extfile',name+'.ext')
 run('openssl','x509','-in',name+'.pem','-outform','DER','-out',name+'.der')
run('openssl','pkcs12','-export','-in','server.pem','-inkey','server.key','-certfile','ca.pem','-name','server','-out','identity.p12','-passout','pass:emulator-only')
(root/'trust.p12').unlink(missing_ok=True)
run('keytool','-importcert','-noprompt','-alias','ca','-file','ca.pem','-keystore','trust.p12','-storetype','PKCS12','-storepass','emulator-only')
(root/'tls-password').write_text('emulator-only')
(root/'client-sha256').write_text(hashlib.sha256((root/'client.der').read_bytes()).hexdigest())
config={'host':'127.0.0.1','port':37179,'server_name':'omarchy-android','server_sha256':hashlib.sha256((root/'server.der').read_bytes()).hexdigest(),'ca':str(root/'ca.pem'),'certificate':str(root/'client.pem'),'private_key':str(root/'client.key')}
(root/'client.json').write_text(json.dumps(config))
for path in root.iterdir():os.chmod(path,0o600)
print('Disposable two-day transport fixture prepared')
