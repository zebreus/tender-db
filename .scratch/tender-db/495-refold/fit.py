import re,sys
def lstsq(X,y):
    # normal equations, small n
    n=len(X[0])
    A=[[sum(r[i]*r[j] for r in X) for j in range(n)] for i in range(n)]
    b=[sum(r[i]*yy for r,yy in zip(X,y)) for i in range(n)]
    # gaussian elimination
    for i in range(n):
        p=max(range(i,n),key=lambda k:abs(A[k][i])); A[i],A[p]=A[p],A[i]; b[i],b[p]=b[p],b[i]
        for k in range(i+1,n):
            f=A[k][i]/A[i][i]
            for j in range(i,n): A[k][j]-=f*A[i][j]
            b[k]-=f*b[i]
    x=[0]*n
    for i in reversed(range(n)):
        x[i]=(b[i]-sum(A[i][j]*x[j] for j in range(i+1,n)))/A[i][i]
    return x
runs=[[]]
for line in open(sys.argv[1]):
    if line.startswith('==='): runs.append([]); continue
    m=re.match(r'(\d+\.\d+) .*phase 2: (\d+)/8780780 tenders folded, (\d+) versions written, (\d+) leaf rows',line)
    if m: runs[-1].append(tuple(float(x) for x in m.groups()))
for name,r in zip(['2067','2953'],runs):
    d=[]
    for a,b in zip(r,r[1:]):
        dt=b[0]-a[0]; T=b[1]-a[1]; V=b[2]-a[2]; R=b[3]-a[3]
        if V<=0 or dt<=0: continue
        d.append((dt,T,V,R))
    dd=d[1:]
    y=[x[0] for x in dd]
    s2=lstsq([[x[2],x[3]] for x in dd],y)
    s3=lstsq([[x[2],x[3],x[1]] for x in dd],y)
    st=lstsq([[x[1],x[3]] for x in dd],y)
    pred=[s2[0]*x[2]+s2[1]*x[3] for x in dd]; my=sum(y)/len(y)
    r2=1-sum((a-b)**2 for a,b in zip(y,pred))/sum((a-my)**2 for a in y)
    predt=[st[0]*x[1]+st[1]*x[3] for x in dd]
    r2t=1-sum((a-b)**2 for a,b in zip(y,predt))/sum((a-my)**2 for a in y)
    tot=sum(x[0] for x in d); R=sum(x[3] for x in d); V=sum(x[2] for x in d); T=sum(x[1] for x in d)
    print(name,'buckets',len(d),'wall',round(tot),'s  T',int(T),'V',int(V),'R',int(R))
    print('  rates: T/s %.0f V/s %.0f R/s %.0f R/T %.0f R/V %.1f'%(T/tot,V/tot,R/tot,R/T,R/V))
    print('  fit dt = %.3f ms/version + %.2f us/leafrow (R2 %.2f); per-version share %.0f%%'%(s2[0]*1e3,s2[1]*1e6,r2,100*s2[0]*V/(s2[0]*V+s2[1]*R)))
    print('  fit dt = %.3f ms/tender + %.2f us/leafrow (R2 %.2f); per-tender share %.0f%%'%(st[0]*1e3,st[1]*1e6,r2t,100*st[0]*T/(st[0]*T+st[1]*R)))
    print('  fit3 dt = %.3f ms/version + %.2f us/leafrow + %.3f ms/tender'%(s3[0]*1e3,s3[1]*1e6,s3[2]*1e3))
    rs=sorted(x[3]/x[0] for x in dd); print('  leaf rows/s per bucket: min %.0f median %.0f max %.0f'%(rs[0],rs[len(rs)//2],rs[-1]))
