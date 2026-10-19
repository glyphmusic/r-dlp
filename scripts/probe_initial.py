import json, time, urllib.request
video='dQw4w9WgXcQ'
client={'clientName':'VISIONOS','clientVersion':'1.02','deviceMake':'Apple','deviceModel':'RealityDevice17,1','osName':'visionOS','osVersion':'26.5.23O471','hl':'en','gl':'US'}
ua='Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15'
start=time.perf_counter()
r=urllib.request.Request('https://www.youtube.com/youtubei/v1/player?prettyPrint=false',json.dumps({'context':{'client':client},'videoId':video,'contentCheckOk':True,'racyCheckOk':True}).encode(),{'Content-Type':'application/json','User-Agent':ua,'X-Youtube-Client-Name':'101','X-Youtube-Client-Version':'1.02'})
try:
 info=json.load(urllib.request.urlopen(r,timeout=15)); print('elapsed',time.perf_counter()-start);print(json.dumps(info.get('playabilityStatus')))
 formats=info.get('streamingData',{}).get('adaptiveFormats',[])
 print('formats',[(x.get('itag'),x.get('mimeType'),bool(x.get('url'))) for x in formats]);print('stream keys',list(info.get('streamingData',{})))
 for x in formats:
  if x.get('mimeType','').startswith('audio/mp4') and x.get('url'):
   rr=urllib.request.urlopen(urllib.request.Request(x['url'],headers={'Range':'bytes=0-1023','User-Agent':ua}),timeout=15);print('audio',rr.status,len(rr.read(1024)));break
except Exception as e:print(type(e).__name__,str(e))
