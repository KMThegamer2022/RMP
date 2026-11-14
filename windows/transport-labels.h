/* Compositors differ in their treatment of GDI text alpha on an extended
   frame. Generate explicit premultiplied ink and glow, as with a 32-bit icon. */
static bool glassLabel(const DRAWITEMSTRUCT *draw,const WCHAR *text){
 HDC target=draw->hDC;RECT r=draw->rcItem;
 int width=r.right-r.left,height=r.bottom-r.top;
 if(width<=0||height<=0)return true;
 BITMAPINFO bi={0};bi.bmiHeader.biSize=sizeof(bi.bmiHeader);bi.bmiHeader.biWidth=width;
 bi.bmiHeader.biHeight=-height;bi.bmiHeader.biPlanes=1;bi.bmiHeader.biBitCount=32;bi.bmiHeader.biCompression=BI_RGB;
 RGBQUAD *pixels=NULL;
 HBITMAP dib=CreateDIBSection(target,&bi,DIB_RGB_COLORS,(void**)&pixels,NULL,0);
 HDC dc=CreateCompatibleDC(target);BYTE *coverage=calloc((size_t)width*height,1);
 bool rendered=false;
 if(dc&&dib&&coverage){
  HGDIOBJ old=SelectObject(dc,dib);RECT local={0,0,width,height};
  FillRect(dc,&local,GetStockObject(BLACK_BRUSH));
  local.left+=scale(3);textIn(dc,local,text,RGB(255,255,255),DT_LEFT);
  GdiFlush();
  for(int i=0;i<width*height;i++)coverage[i]=(BYTE)((pixels[i].rgbRed+2*pixels[i].rgbGreen+pixels[i].rgbBlue)/4);
  int radius=max(1,scale(2));
  for(int y=0;y<height;y++)for(int x=0;x<width;x++){
   int i=y*width+x,ink=coverage[i],halo=0;
   for(int dy=-radius;dy<=radius;dy++)for(int dx=-radius;dx<=radius;dx++){
    int xx=x+dx,yy=y+dy;
    if(xx>=0&&yy>=0&&xx<width&&yy<height){
     int fringe=coverage[yy*width+xx]*max(0,2*radius+1-abs(dx)-abs(dy))/(2*radius+1);
     if(fringe>halo)halo=fringe;
    }
   }
   int glow=(255-ink)*halo/255;
   pixels[i]=(RGBQUAD){(BYTE)(65*ink/255+glow),(BYTE)(46*ink/255+glow),
                       (BYTE)(18*ink/255+glow),(BYTE)(ink+glow)};
  }
  FillRect(target,&r,GetStockObject(BLACK_BRUSH));
  BLENDFUNCTION alpha={AC_SRC_OVER,0,255,AC_SRC_ALPHA};
  rendered=AlphaBlend(target,r.left,r.top,width,height,dc,0,0,width,height,alpha)!=0;
  SelectObject(dc,old);
 }
 free(coverage);if(dc)DeleteDC(dc);if(dib)DeleteObject(dib);
 return rendered;
}
static void paintTransportLabel(const DRAWITEMSTRUCT *draw){
 WCHAR text[100];GetWindowTextW(draw->hwndItem,text,100);
 if(glass && !contrastMode && glassLabel(draw,text))return;
 RECT r=draw->rcItem;
 if(skin==XP&&!contrastMode)xpControlBackdrop(draw->hwndItem,draw->hDC);
 else FillRect(draw->hDC,&r,contrastMode?GetSysColorBrush(COLOR_BTNFACE):panelBrush);
 textIn(draw->hDC,r,text,contrastMode?GetSysColor(COLOR_BTNTEXT):color(themes[skin].text),DT_LEFT);
}
