#!/usr/bin/env python3
import base64,json,subprocess,sys,tempfile,zipfile
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'transport'))
from enroll import issue_bundle
with tempfile.TemporaryDirectory() as temp:
    root=Path(temp)
    subprocess.run(['openssl','genpkey','-algorithm','EC','-pkeyopt','ec_paramgen_curve:P-256','-out',str(root/'key')],check=True,capture_output=True)
    public=subprocess.check_output(['openssl','pkey','-in',str(root/'key'),'-pubout','-outform','DER'])
    (root/'public.der').write_bytes(public)
    (root/'request.json').write_text(json.dumps({'v':1,'request_id':'a'*32,'public_key':base64.b64encode(public).decode()}))
    issue_bundle(root/'request.json',root/'issued',days=2)
    bad=root/'bad';bad.mkdir()
    original={}
    with zipfile.ZipFile(root/'issued/phone-enrollment.zip') as bundle:
        original={n:bundle.read(n) for n in bundle.namelist()}
    def save(name,parts):
        with zipfile.ZipFile(bad/(name+'.zip'),'w') as output:
            for filename,value in parts.items():output.writestr(filename,value)
    save('unknown-entry',dict(original,unexpected=b'bad'))
    save('oversize-entry',dict(original,**{'client.der':b'x'*17000}))
    save('trailing-certificate',dict(original,**{'client.der':original['client.der']+b'junk'}))
    save('wrong-client-purpose',dict(original,**{'client.der':original['server.der']}))
    save('ca-as-leaf',dict(original,**{'client.der':original['ca.der']}))
    # Same phone key, signed by the expected CA, but an unapproved server name.
    ext=root/'wrong-san.ext';ext.write_text('basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:wrong.invalid\n')
    certificate=root/'wrong-san.der'
    subprocess.run(['openssl','x509','-new','-force_pubkey',str(root/'issued/phone-public.pem'),'-CA',str(root/'issued/ca.pem'),'-CAkey',str(root/'issued/ca.key'),'-set_serial','99','-days','2','-subj','/CN=wrong.invalid','-extfile',str(ext),'-outform','DER','-out',str(certificate)],check=True,capture_output=True)
    save('wrong-san',dict(original,**{'server.der':certificate.read_bytes()}))
    subprocess.run(['javac' ,'-d',str(root),'framework/bridge/EnrollmentBundle.java','framework/unit/EnrollmentBundleTest.java'],check=True)
    subprocess.run(['java','-cp',str(root),'org.omarchy.bridge.EnrollmentBundleTest',str(root/'issued/phone-enrollment.zip'),str(root/'public.der'),str(bad)],check=True)
