import numpy as np
from PIL import Image, ImageFilter
cut=Image.open('formsculptor_bust_cutout.png').convert('RGBA')
bb=cut.getbbox(); cut=cut.crop(bb)
a=np.asarray(cut).astype(float)/255; A=a[...,3]
L=0.2126*a[...,0]+0.7152*a[...,1]+0.0722*a[...,2]
m=A>0.5; lo,hi=np.percentile(L[m],2),np.percentile(L[m],99.5)
s=np.clip((L-lo)/(hi-lo),0,1)
H,W=L.shape; yy,xx=np.mgrid[0:H,0:W]
t=np.clip(0.5+0.5*((xx/W-0.5)*0.6+(0.5-yy/H)*1.0)/0.8,0,1)
stops=np.array([[0,0x3A,0x7B,0xFF],[0.32,0x9A,0x5C,0xFF],[0.64,0xFF,0x4F,0x7A],[1,0xFF,0xAE,0x2B]],float)
base=np.stack([np.interp(t,stops[:,0],stops[:,i]) for i in (1,2,3)],-1)/255
shade=0.34+0.86*s**1.05
col=base*shade[...,None]+0.24*(s**5)[...,None]
out=np.dstack([np.clip(col,0,1),A]); Image.fromarray((out*255).astype(np.uint8),'RGBA').save('bust_color.png')
print(cut.size)
