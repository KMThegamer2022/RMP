/* Paint native theme parts into a 32-bit surface. Untouched pixels expose DWM. */
static bool paintGlassControl(HWND h,HDC dc){
 bool slider=h==position||h==level;
 bool label=h==volumeLabel||h==clocktext;
 HTHEME theme=OpenThemeData(h,slider?L"Trackbar":label?L"Window":L"Toolbar");
 if(!theme)return false;
 RECT r;GetClientRect(h,&r);
 BP_PAINTPARAMS params={sizeof(params),BPPF_ERASE};
 HDC surface;
 HPAINTBUFFER buffer=BeginBufferedPaint(dc,&r,BPBF_TOPDOWNDIB,&params,&surface);
 if(!buffer){CloseThemeData(theme);return false;}
 HRESULT result=S_OK;
 bool enabled=IsWindowEnabled(h);
 POINT cursor;GetCursorPos(&cursor);ScreenToClient(h,&cursor);
 bool hot=PtInRect(&r,cursor);
 if(slider){
  RECT channel,thumb;
  SendMessageW(h,TBM_GETCHANNELRECT,0,(LPARAM)&channel);
  SendMessageW(h,TBM_GETTHUMBRECT,0,(LPARAM)&thumb);
  result=DrawThemeBackground(theme,surface,TKP_TRACK,TRS_NORMAL,&channel,NULL);
  int state=!enabled?TUS_DISABLED:GetCapture()==h?TUS_PRESSED:PtInRect(&thumb,cursor)?TUS_HOT:GetFocus()==h?TUS_FOCUSED:TUS_NORMAL;
  if(SUCCEEDED(result))result=DrawThemeBackground(theme,surface,TKP_THUMBBOTTOM,state,&thumb,NULL);
 }else{
  int state=!enabled?TS_DISABLED:(SendMessageW(h,BM_GETSTATE,0,0)&BST_PUSHED)?TS_PRESSED:hot?TS_HOT:TS_NORMAL;
  if(!label && state!=TS_NORMAL)result=DrawThemeBackground(theme,surface,TP_BUTTON,state,&r,NULL);
  WCHAR text[100];GetWindowTextW(h,text,100);
  SelectObject(surface,uiFont);
  DTTOPTS opts={sizeof(opts)};
  opts.dwFlags=DTT_COMPOSITED|DTT_TEXTCOLOR|DTT_GLOWSIZE;
  opts.crText=GetSysColor(enabled?COLOR_BTNTEXT:COLOR_GRAYTEXT);
  opts.iGlowSize=scale(5);
  if(SUCCEEDED(result))result=DrawThemeTextEx(theme,surface,label?0:TP_BUTTON,label?0:state,text,-1,
      DT_SINGLELINE|DT_VCENTER|DT_NOPREFIX|(label?DT_LEFT:DT_CENTER),&r,&opts);
 }
 if(!label && GetFocus()==h && !(SendMessageW(h,WM_QUERYUISTATE,0,0)&UISF_HIDEFOCUS)){
  RECT focus=r;InflateRect(&focus,-2,-2);
  // Give the keyboard focus outline a visible alpha channel on glass.
  FrameRect(surface,&focus,GetSysColorBrush(COLOR_HIGHLIGHT));
  RECT edges[]={ {focus.left,focus.top,focus.right,focus.top+1},{focus.left,focus.bottom-1,focus.right,focus.bottom},
                 {focus.left,focus.top,focus.left+1,focus.bottom},{focus.right-1,focus.top,focus.right,focus.bottom} };
  for(int i=0;i<4;i++)BufferedPaintSetAlpha(buffer,&edges[i],255);
 }
 EndBufferedPaint(buffer,SUCCEEDED(result));CloseThemeData(theme);
 return SUCCEEDED(result);
}
static LRESULT CALLBACK glassControlProc(HWND h,UINT msg,WPARAM wp,LPARAM lp,UINT_PTR id,DWORD_PTR data){
 if(glass && GetParent(h)==mainwin){
  if(msg==WM_ERASEBKGND)return 1;
  if(msg==WM_PAINT){
   PAINTSTRUCT ps;HDC dc=BeginPaint(h,&ps);
   if(!paintGlassControl(h,dc))DefSubclassProc(h,WM_PRINTCLIENT,(WPARAM)dc,PRF_CLIENT|PRF_ERASEBKGND);
   EndPaint(h,&ps);return 0;
  }
  if(msg==WM_PRINTCLIENT && paintGlassControl(h,(HDC)wp))return 0;
 }
 return DefSubclassProc(h,msg,wp,lp);
}
