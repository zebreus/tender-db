import re,sys
runs=[[]]
for line in open(sys.argv[1]):
    if line.startswith('==='): runs.append([]); continue
    m=re.match(r'(\d+\.\d+) .*phase 2: (\d+)/8780780 tenders folded, (\d+) versions written, (\d+) leaf rows',line)
    if m: runs[-1].append(tuple(float(x) for x in m.groups()))
for name,r in zip(['2067','2953'],runs):
    print(name)
    step=30 if name=='2953' else 8
    for i in range(0,len(r)-1,step):
        a=r[i]; b=r[min(i+step,len(r)-1)]
        dt=b[0]-a[0]; T=b[1]-a[1]; V=b[2]-a[2]; R=b[3]-a[3]
        if dt<=0: continue
        print('  buckets %3d-%3d  %6.0fs  T/s %5.0f  V/s %4.0f  R/s %6.0f  R/T %5.0f  V/T %.2f  us/row %.1f  ms/tender %.2f'%(i,i+step,dt,T/dt,V/dt,R/dt,R/T,V/T,1e6*dt/R,1e3*dt/T))
