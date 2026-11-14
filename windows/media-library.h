/* XP Now Playing and the Windows 7 library share the actual player queue. */
enum { NAV_NOW=510, NAV_LIBRARY, NAV_QUEUE, LOAD_LIST, SAVE_LIST, FILTER, TRACKS, ADD_FILE };
static const int browserButtons[]={NAV_NOW,NAV_LIBRARY,NAV_QUEUE,ADD_FILE,LOAD_LIST,SAVE_LIST};
static const WCHAR *browserTitles[]={L"Now Playing",L"Media Library",L"Current Playlist",L"Open media…",L"Open playlist…",L"Save playlist…"};
static HWND mediaRows,mediaFilter;
static int browserPage=NAV_NOW;
static int lastQueueMark=-2;
static const WCHAR *sectionTitle(void){
 return browserPage==NAV_LIBRARY?L"Media Library":browserPage==NAV_QUEUE?L"Current Playlist":L"Now Playing";
}
static bool mediaTheme(void){return skin==XP||skin==SEVEN;}
static void setBrowserPart(HWND h,bool visible,int x,int y,int width,int height){
 if(!h)return;
 if(visible)MoveWindow(h,x,y,max(1,width),max(1,height),TRUE);
 ShowWindow(h,visible?SW_SHOWNA:SW_HIDE);
}
static void makeMediaBrowser(void){
 for(size_t i=0;i<sizeof(browserButtons)/sizeof(*browserButtons);i++){
  CreateWindowW(L"BUTTON",browserTitles[i],WS_CHILD|WS_TABSTOP|BS_OWNERDRAW,0,0,1,1,mainwin,
   (HMENU)(INT_PTR)browserButtons[i],GetModuleHandleW(NULL),NULL);
 }
 mediaFilter=CreateWindowExW(WS_EX_CLIENTEDGE,L"EDIT",L"",WS_CHILD|WS_TABSTOP|ES_AUTOHSCROLL,
  0,0,1,1,mainwin,(HMENU)(INT_PTR)FILTER,GetModuleHandleW(NULL),NULL);
 SendMessageW(mediaFilter,EM_SETCUEBANNER,TRUE,(LPARAM)L"Search tracks");
 mediaRows=CreateWindowExW(WS_EX_CLIENTEDGE,WC_LISTVIEWW,L"Tracks",WS_CHILD|WS_TABSTOP|LVS_REPORT|LVS_SINGLESEL|LVS_SHOWSELALWAYS,
  0,0,1,1,mainwin,(HMENU)(INT_PTR)TRACKS,GetModuleHandleW(NULL),NULL);
 ListView_SetExtendedListViewStyle(mediaRows,LVS_EX_FULLROWSELECT|LVS_EX_DOUBLEBUFFER|LVS_EX_LABELTIP);
 const WCHAR *columns[]={L"Title",L"Type",L"Location"};
 for(int i=0;i<3;i++){
  LVCOLUMNW col={0};col.mask=LVCF_TEXT|LVCF_WIDTH|LVCF_SUBITEM;col.iSubItem=i;
  col.cx=scale(i==0?230:100);col.pszText=(WCHAR*)columns[i];
  SendMessageW(mediaRows,LVM_INSERTCOLUMNW,i,(LPARAM)&col);
 }
}
static bool containsTitle(const WCHAR *value,const WCHAR *query){
 if(!*query)return true;
 int length=lstrlenW(query),remaining=lstrlenW(value);
 for(int pos=0;pos+length<=remaining;pos++)
  if(CompareStringOrdinal(value+pos,length,query,length,TRUE)==CSTR_EQUAL)return true;
 return false;
}
static void markPlayingInQueue(void){
 if(browserPage==NAV_LIBRARY||!IsWindowVisible(mediaRows))return;
 int current=(int)ap_index(player);
 if(current==lastQueueMark)return;
 lastQueueMark=current;
 int rows=ListView_GetItemCount(mediaRows);
 for(int n=0;n<rows;n++){
  LVITEMW row={0};row.mask=LVIF_PARAM;row.iItem=n;
  SendMessageW(mediaRows,LVM_GETITEMW,0,(LPARAM)&row);
  if((int)row.lParam==current){ListView_SetItemState(mediaRows,n,LVIS_SELECTED,LVIS_SELECTED);break;}
 }
}
static void rebuildMediaRows(void){
 if(!mediaRows || !mediaTheme())return;
 EnableWindow(GetDlgItem(mainwin,SAVE_LIST),ap_count(player)>0);
 if(fullscreen||(skin==SEVEN&&browserPage==NAV_NOW))return;
 bool library=browserPage==NAV_LIBRARY;
 WCHAR query[256]={0};
 if(browserPage!=NAV_NOW)GetWindowTextW(mediaFilter,query,256);
 size_t count=library?ap_library_count(player):ap_count(player);
 SendMessageW(mediaRows,WM_SETREDRAW,FALSE,0);
 ListView_DeleteAllItems(mediaRows);
 for(size_t i=0;i<count;i++){
  const char *source=library?ap_library_item(player,i):ap_item(player,i);
  if(!source)continue;
  WCHAR *path=wide(source);
  if(!path)continue;
  WCHAR *title=path;
  for(WCHAR *p=path;*p;p++)if(*p==L'\\'||*p==L'/')title=p+1;
  if(containsTitle(path,query)){
   LVITEMW row={0};row.mask=LVIF_TEXT|LVIF_PARAM;row.iItem=ListView_GetItemCount(mediaRows);
   row.pszText=*title?title:path;row.lParam=(LPARAM)i;
   int item=(int)SendMessageW(mediaRows,LVM_INSERTITEMW,0,(LPARAM)&row);
   if(item>=0){
    WCHAR *extension=wcsrchr(title,L'.');
    row.iItem=item;row.iSubItem=1;row.pszText=wcsstr(path,L"://")?L"Stream":extension&&extension[1]?extension+1:L"File";
    SendMessageW(mediaRows,LVM_SETITEMTEXTW,item,(LPARAM)&row);
    row.iSubItem=2;row.pszText=path;SendMessageW(mediaRows,LVM_SETITEMTEXTW,item,(LPARAM)&row);
   }
  }
  free(path);
 }
 SendMessageW(mediaRows,WM_SETREDRAW,TRUE,0);
 InvalidateRect(mediaRows,NULL,TRUE);lastQueueMark=-2;markPlayingInQueue();
}
static void layoutMediaBrowser(int width,int deck){
 bool visible=mediaTheme()&&!fullscreen;
 bool xp=skin==XP,playing=browserPage==NAV_NOW;
 int rail=scale(xp?124:148),heading=scale(xp?40:42),tools=scale(36);
 int bodyTop=xp?heading:heading+tools;
 for(int i=0;i<3;i++){
  int id=xp?browserButtons[i]:(i==0?NAV_LIBRARY:i==1?NAV_QUEUE:NAV_NOW);
  setBrowserPart(GetDlgItem(mainwin,id),visible,scale(7),bodyTop+scale(12+i*(xp?45:33)),
     rail-scale(14),scale(xp?44:32));
 }
 for(int i=0;i<3;i++){
  bool compact=xp && deck-bodyTop<scale(250);
  int x=compact?rail+scale(i*91):xp?scale(7):scale(8+i*116);
  int y=compact?scale(6):xp?bodyTop+scale(156+i*32):heading+scale(3);
  setBrowserPart(GetDlgItem(mainwin,browserButtons[3+i]),visible,x,y,compact?scale(89):xp?rail-scale(14):scale(112),scale(29));
 }
 int filterWidth=scale(186);
 setBrowserPart(mediaFilter,visible&&!playing,width-filterWidth-scale(10),xp?scale(8):heading+scale(5),filterWidth,scale(25));
 int queueWidth=xp&&playing?min(scale(224),width/4):0;
 int listLeft=playing?width-queueWidth-scale(5):rail+scale(3);
 int listTop=playing?bodyTop+scale(28):bodyTop+scale(2);
 bool table=visible&&(!playing||xp);
 setBrowserPart(mediaRows,table,listLeft,listTop,width-listLeft-scale(5),deck-listTop-scale(5));
 if(table){
  int area=width-listLeft-scale(12);
  ListView_SetColumnWidth(mediaRows,0,playing?area:max(scale(110),area*43/100));
  ListView_SetColumnWidth(mediaRows,1,playing?0:scale(74));
  ListView_SetColumnWidth(mediaRows,2,playing?0:max(scale(70),area-area*43/100-scale(80)));
 }
 if(visible){
  setBrowserPart(video,playing,rail+scale(3),bodyTop+scale(2),width-rail-queueWidth-scale(9),deck-bodyTop-scale(7));
  if(playing){RECT v;GetClientRect(video,&v);int logo=scale(88);MoveWindow(idleLogo,(v.right-logo)/2,(v.bottom-logo)/2,logo,logo,TRUE);}
 }
}
static void showMediaPage(int page){
 browserPage=page;
 SetWindowTextW(mediaFilter,L"");
 RECT r;GetClientRect(mainwin,&r);layout(mainwin,r.right,r.bottom);
 rebuildMediaRows();RedrawWindow(mainwin,NULL,NULL,RDW_INVALIDATE|RDW_ERASE|RDW_ALLCHILDREN);
}
static void playMediaRow(void){
 int index=ListView_GetNextItem(mediaRows,-1,LVNI_SELECTED);
 if(index<0)return;
 LVITEMW row={0};row.mask=LVIF_PARAM;row.iItem=index;
 if(!SendMessageW(mediaRows,LVM_GETITEMW,0,(LPARAM)&row))return;
 if(browserPage==NAV_LIBRARY){
  if(ap_library_play(player,(size_t)row.lParam))showCurrentTrack();
  else MessageBoxW(mainwin,L"Could not play the selected library track.",L"RMP",MB_ICONERROR);
 }else start((size_t)row.lParam);
}
static void playlistFile(bool save){
 WCHAR filename[32768]={0};OPENFILENAMEW dlg={sizeof(dlg)};
 dlg.hwndOwner=mainwin;dlg.lpstrFile=filename;dlg.nMaxFile=32768;
 dlg.lpstrFilter=L"UTF-8 playlists (*.m3u8, *.m3u)\0*.m3u8;*.m3u\0All files\0*.*\0";dlg.lpstrDefExt=L"m3u8";
 dlg.Flags=OFN_NOCHANGEDIR|OFN_PATHMUSTEXIST|(save?OFN_OVERWRITEPROMPT:OFN_FILEMUSTEXIST);
 if(!(save?GetSaveFileNameW(&dlg):GetOpenFileNameW(&dlg)))return;
 int bytes=WideCharToMultiByte(CP_UTF8,0,filename,-1,NULL,0,NULL,NULL);char *path=malloc(bytes);
 if(!path)return;
 WideCharToMultiByte(CP_UTF8,0,filename,-1,path,bytes,NULL,NULL);
 bool ok=save?ap_playlist_save(player,path):ap_playlist_load(player,path);free(path);
 if(!ok)MessageBoxW(mainwin,save?L"Could not save this playlist.":L"Could not read this playlist.",L"RMP",MB_ICONERROR);
 else if(!save){if(ap_count(player))start(0);else showMediaPage(NAV_QUEUE);}
}
static void styleMediaRows(void){
 if(!mediaRows||!mediaTheme())return;
 bool violet=skin==XP&&!contrastMode;
 COLORREF bg=contrastMode?GetSysColor(COLOR_WINDOW):color(violet?0x39466e:0xf1f6fc);
 ListView_SetBkColor(mediaRows,bg);ListView_SetTextBkColor(mediaRows,bg);
 ListView_SetTextColor(mediaRows,contrastMode?GetSysColor(COLOR_WINDOWTEXT):color(violet?0xffffff:0x183248));
 SetWindowTheme(mediaRows,skin==SEVEN?L"Explorer":L"",NULL);
}
static void textIn(HDC dc,RECT rect,const WCHAR *text,COLORREF ink,UINT format){
 SetBkMode(dc,TRANSPARENT);SetTextColor(dc,ink);SelectObject(dc,uiFont);
 DrawTextW(dc,text,-1,&rect,DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS|DT_NOPREFIX|format);
}
static void paintBrowserButton(DRAWITEMSTRUCT *d){
 RECT r=d->rcItem;HDC dc=d->hDC;int id=d->CtlID;
 bool xp=skin==XP,chosen=id==browserPage,down=d->itemState&ODS_SELECTED;
 if(contrastMode){FillRect(dc,&r,GetSysColorBrush(chosen?COLOR_HIGHLIGHT:COLOR_BTNFACE));}
 else if(xp){
  blend(dc,r,chosen||down?0xabc6ee:0xf0f5ff,chosen||down?0xe1eeff:0xc0c9e1);
  SetDCBrushColor(dc,color(0x8fa8d0));FrameRect(dc,&r,GetStockObject(DC_BRUSH));
 }else if(chosen||down){blend(dc,r,0xe9f4fc,0xc0dcef);SetDCBrushColor(dc,color(0x9dbbd0));FrameRect(dc,&r,GetStockObject(DC_BRUSH));}
 else if(id==NAV_NOW||id==NAV_LIBRARY||id==NAV_QUEUE)fillColor(dc,r,color(0xe6eff8));
 else blend(dc,r,0xf3f8fd,0xd9e6f3);
 WCHAR caption[48];GetWindowTextW(d->hwndItem,caption,48);
 RECT label=r;label.left+=scale(8);textIn(dc,label,caption,contrastMode?GetSysColor(chosen?COLOR_HIGHLIGHTTEXT:COLOR_BTNTEXT):color(0x19395a),DT_LEFT);
 if(d->itemState&ODS_FOCUS){InflateRect(&r,-2,-2);DrawFocusRect(dc,&r);}
}
static void paintMediaSurface(HDC dc,RECT r){
 bool xp=skin==XP;int rail=scale(xp?124:148),heading=scale(xp?40:42),deck=r.bottom-scale(MEDIA_TRANSPORT_HEIGHT);
 fillColor(dc,r,color(xp?0x6472a1:0xe7f0f9));
 RECT head={0,0,r.right,heading};blend(dc,head,xp?0xf4f6ff:0xf8fbff,xp?0x7587bc:0xdae6f3);
 RECT title=head;title.left=scale(14);
 WCHAR breadcrumb[128];swprintf(breadcrumb,128,xp?L"RMP   •   %ls":L"Library   ›   %ls",sectionTitle());
 textIn(dc,title,breadcrumb,color(0x153158),DT_LEFT);
 int top=heading;
 if(!xp){RECT barrect={0,heading,r.right,heading+scale(36)};blend(dc,barrect,0xf3f8fd,0xd9e6f3);top=barrect.bottom;}
 RECT nav={scale(xp?4:0),top,rail,deck};blend(dc,nav,xp?0xc1d3ef:0xe6eff8,xp?0x6e81b8:0xe6eff8);
 if(xp&&browserPage==NAV_NOW){
  RECT q={r.right-min(scale(224),r.right/4)-scale(5),top+scale(2),r.right-scale(5),top+scale(28)};
  fillColor(dc,q,color(0x52618f));q.left+=scale(8);textIn(dc,q,L"Current Playlist",RGB(255,255,255),DT_LEFT);
 }
 RECT footer={0,deck,r.right,r.bottom};
 if(glass){FillRect(dc,&footer,GetStockObject(BLACK_BRUSH));}
 else {blend(dc,footer,xp?0x8594ba:0xe9f2fa,xp?0xe1eaf8:0xd2e3f0);
  if(xp){RECT shine=footer;shine.bottom=shine.top+scale(8);blend(dc,shine,0xfaFCff,0xb5c5e1);}
 }
}
static void layoutClassicTransport(HWND host,int width,int deck){
 bool xp=skin==XP;
 int x=xp?scale(16):(width-scale(350))/2;
 const int sevenOrder[]={SHUFFLE,REPEAT,STOP,PREV,PLAY,NEXT};
 const int xpOrder[]={PLAY,STOP,PREV,NEXT,SHUFFLE,REPEAT};
 const int *order=xp?xpOrder:sevenOrder;
 int center=deck+scale(54);
 ShowWindow(GetDlgItem(host,OPEN),SW_HIDE);ShowWindow(GetDlgItem(host,LIST),SW_HIDE);
 for(int i=0;i<6;i++){
  int id=order[i],bw=id==PLAY?scale(44):scale(xp?30:36),bh=id==PLAY?scale(42):scale(28);
  setBrowserPart(GetDlgItem(host,id),true,x,center-bh/2,bw,bh);x+=bw+scale(3);
 }
 setBrowserPart(volumeLabel,true,x+scale(4),center-scale(12),scale(47),scale(24));
 setBrowserPart(level,true,x+scale(52),center-scale(12),scale(77),scale(24));
 setBrowserPart(GetDlgItem(host,FULL),true,width-scale(47),center-scale(14),scale(36),scale(28));
 MoveWindow(position,scale(9),deck+scale(2),width-scale(140),scale(24),TRUE);
 MoveWindow(clocktext,width-scale(126),deck+scale(2),scale(122),scale(24),TRUE);
}
static void paintXpTransport(DRAWITEMSTRUCT *d){
 RECT r=d->rcItem;HDC dc=d->hDC;fillColor(dc,r,color(0xc5d1e7));InflateRect(&r,-1,-1);
 HRGN oval=CreateEllipticRgnIndirect(&r);
 int saved=SaveDC(dc);ExtSelectClipRgn(dc,oval,RGN_AND);
 blend(dc,r,0xfefefe,(d->itemState&ODS_SELECTED)?0x91a7c5:0x91aac2);
 RECT gloss=r;gloss.bottom=(r.top+r.bottom)/2;blend(dc,gloss,0xffffff,0xc5d6e6);
 RestoreDC(dc,saved);SetDCBrushColor(dc,color(0x647a96));FrameRgn(dc,oval,GetStockObject(DC_BRUSH),1,1);DeleteObject(oval);
 WCHAR text[40];GetWindowTextW(d->hwndItem,text,40);
 const WCHAR *icon=d->CtlID==PLAY?(wcsstr(text,L"Pause")?L"Ⅱ":L"▶"):d->CtlID==PREV?L"❘◀":d->CtlID==NEXT?L"▶❘":text;
 textIn(dc,r,icon,color(0x173c60),DT_CENTER);
 if((d->itemState&ODS_FOCUS)||(d->CtlID==REPEAT&&repeatOne)||(d->CtlID==SHUFFLE&&shuffle)){
  InflateRect(&r,-scale(4),-scale(4));DrawFocusRect(dc,&r);
 }
}
