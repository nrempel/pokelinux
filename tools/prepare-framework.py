import pathlib, urllib.request, hashlib, zipfile, json
root=pathlib.Path('android-framework'); sdk=root/'sdk'; sdk.mkdir(exist_ok=True)
items=[('emulator','https://dl.google.com/android/repository/emulator-linux_x64-16428233.zip','cd7362ea55dfb86a418958138dc396e74165dd01','.'),('platform-tools','https://dl.google.com/android/repository/platform-tools_r37.0.1-linux.zip','477254aa5f903c15cf51001717bdf347fb6b53e0','.'),('platform','https://dl.google.com/android/repository/platform-37.0_r02.zip','ed8ebf7f8822a4de5686d427f237d2fa30ff7410','platform'),('image','https://dl.google.com/android/repository/sys-img/google_apis/x86_64-37.0_r06.zip','629e507fd5b737c2c836b12b52c81cd0e3b12399','system-image')]
records=[]
for name,url,sha,dest in items:
 p=root/url.rsplit('/',1)[1]
 if not p.exists():
  print('download',name,flush=True); urllib.request.urlretrieve(url,p)
 data=p.read_bytes(); assert hashlib.sha1(data).hexdigest()==sha,name
 records.append(dict(name=name,url=url,sha1=sha,sha256=hashlib.sha256(data).hexdigest(),size=len(data)))
 del data
 out=sdk/dest; out.mkdir(exist_ok=True)
 with zipfile.ZipFile(p) as z:
  z.extractall(out)
  for i in z.infolist():
   mode=i.external_attr>>16
   if mode: (out/i.filename).chmod(mode & 0o777)
 print('verified/extracted',name,flush=True)
assert hashlib.sha256(pathlib.Path('audit/build-tools_r37_linux.zip').read_bytes()).hexdigest()=='01af179347cbcd9c208b7f8171f7b21f6dd1d2f85bcd15e88caa51d5d7b86060'
with zipfile.ZipFile('audit/build-tools_r37_linux.zip') as z:
 z.extractall(sdk/'build-tools')
 for i in z.infolist():
  mode=i.external_attr>>16
  if mode:(sdk/'build-tools'/i.filename).chmod(mode & 0o777)
(root/'downloads.json').write_text(json.dumps(records,indent=2)+'\n')
# System APIs are needed only for the reference IMS lifecycle fixture.
import base64
url='https://android.googlesource.com/platform/prebuilts/sdk/+/refs/heads/android17-release/current/system/android.jar?format=TEXT'
p=sdk/'system-android17.jar'
if not p.exists():
    p.write_bytes(base64.b64decode(urllib.request.urlopen(url,timeout=90).read()))
data=p.read_bytes()
blob=hashlib.sha1(b'blob '+str(len(data)).encode()+b'\0'+data).hexdigest()
assert blob=='13f165f7627efa43e3f2cdb1e3e463d010c2e2ad','Android 17 system SDK changed; do not silently use new APIs'
(root/'system-sdk.json').write_text(json.dumps(dict(url=url,git_blob=blob,sha256=hashlib.sha256(data).hexdigest(),size=len(data)),indent=2)+'\n')
