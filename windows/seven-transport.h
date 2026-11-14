/* Resource order matches the PNG artwork exported by packaging/render-seven.m. */
static const int sevenIds[]={PLAY,PREV,STOP,NEXT,FULL,LIST,REPEAT,SHUFFLE};
static const int sevenWidths[]={34,70,30,46,30,58,30,28};
static HICON sevenIcons[9];
static void paintSevenFace(DRAWITEMSTRUCT *d){
 int index=0;
 while(index<8 && sevenIds[index]!=(int)d->CtlID)index++;
 if(index==8)return;
 if(index==0){WCHAR title[32];GetWindowTextW(d->hwndItem,title,32);if(wcsstr(title,L"Pause"))index=8;}
 RECT r=d->rcItem;
 int width=r.right-r.left,height=r.bottom-r.top;
 if(!sevenIcons[index]){
  HMODULE module=GetModuleHandleW(NULL);
  HRSRC res=FindResourceW(module,MAKEINTRESOURCEW(410+index),MAKEINTRESOURCEW(10));
  if(res)sevenIcons[index]=CreateIconFromResourceEx(LockResource(LoadResource(module,res)),SizeofResource(module,res),
     TRUE,0x00030000,width,height,0);
 }
 if(!sevenIcons[index]){paintButton(d);return;}
 FillRect(d->hDC,&r,glass?GetStockObject(BLACK_BRUSH):panelBrush);
 int offset=(d->itemState&ODS_SELECTED)?scale(1):0;
 DrawIconEx(d->hDC,offset,offset,sevenIcons[index],width,height,0,NULL,DI_NORMAL);
 if((d->itemState&ODS_FOCUS) || (d->CtlID==REPEAT&&repeatOne) || (d->CtlID==SHUFFLE&&shuffle)){
  InflateRect(&r,-scale(3),-scale(3));DrawFocusRect(d->hDC,&r);
 }
}
