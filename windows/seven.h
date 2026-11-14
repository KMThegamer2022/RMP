/* Premultiplied BGRA carries the tint to DWM. Even over a black or
   unavailable backdrop the tint remains visible. Tiles share window coordinates. */
static HDC sheenDC;
static HBITMAP sheenBitmap,oldSheenBitmap;
static bool loadSheen(void){
 if(sheenDC)return true;
 BITMAPINFO info={0};info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);
 info.bmiHeader.biWidth=384;info.bmiHeader.biHeight=-128;
 info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;
 DWORD *pixels=NULL;
 sheenBitmap=CreateDIBSection(NULL,&info,DIB_RGB_COLORS,(void**)&pixels,NULL,0);
 if(!sheenBitmap)return false;
 sheenDC=CreateCompatibleDC(NULL);
 if(!sheenDC){DeleteObject(sheenBitmap);sheenBitmap=NULL;return false;}
 oldSheenBitmap=SelectObject(sheenDC,sheenBitmap);
 for(int y=0;y<128;y++)for(int x=0;x<384;x++){
 int diagonal=(x+y*2)%384;
 int reflection=max(0,48-abs(diagonal-100))+max(0,18-abs(diagonal-184));
 int gloss=y<59?26-y/3:5+(y-59)/6;
 int r=min(255,146+gloss+reflection),g=min(255,183+gloss+reflection),b=min(255,207+gloss+reflection);
 int alpha=193+reflection/2;
 pixels[y*384+x]=(DWORD)alpha<<24|(DWORD)(r*alpha/255)<<16|(DWORD)(g*alpha/255)<<8|(b*alpha/255);
 }
 return true;
}
static void freeSheen(void){if(sheenDC){SelectObject(sheenDC,oldSheenBitmap);DeleteObject(sheenBitmap);DeleteDC(sheenDC);sheenDC=NULL;}}
static void drawGlassBand(HDC dc,RECT band,bool child){
 if(IsRectEmpty(&band))return;
 if(!glass||!loadSheen()){blend(dc,band,0xe8f6ff,0xb4cfe4);return;}
 BLENDFUNCTION mode={AC_SRC_OVER,0,255,AC_SRC_ALPHA};
 fillColor(dc,band,child?RGB(235,246,255):RGB(0,0,0));int saved=SaveDC(dc);IntersectClipRect(dc,band.left,band.top,band.right,band.bottom);
 for(int x=band.left;x<band.right;x+=384)AlphaBlend(dc,x,band.top,384,band.bottom-band.top,sheenDC,0,0,384,128,mode);
 RestoreDC(dc,saved);
}
static void sevenSurface(HDC dc,HWND child){
 RECT area;GetClientRect(mainwin,&area);
 POINT offset={0};if(child!=mainwin)MapWindowPoints(child,mainwin,&offset,1);
 int save=SaveDC(dc);SetViewportOrgEx(dc,-offset.x,-offset.y,NULL);
 fillColor(dc,area,color(0xbdd4e7));
 RECT deck=area;deck.top=deck.bottom-scale(100);
 drawGlassBand(dc,deck,child!=mainwin);
 RECT line=deck;line.bottom=line.top+1;fillColor(dc,line,RGB(255,255,255));
 RECT edge=area;InflateRect(&edge,-1,-1);SetDCBrushColor(dc,color(0xf3fbff));FrameRect(dc,&edge,GetStockObject(DC_BRUSH));
 RestoreDC(dc,save);
}
