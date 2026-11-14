/* Ask the shell for embedded artwork and tags only for visible library rows.
   Workers never touch HWND controls or the Rust player. UI updates are tagged
   with the table generation so a pending search cannot update the wrong row. */
enum { MEDIA_DETAILS_READY=WM_APP+44, COVER_LIMIT=96 };
typedef struct {
 HWND owner;
 unsigned generation;
 int row,edge;
 WCHAR *path,*title,*album,*artist;
 WCHAR duration[32];
 HBITMAP cover;
} MediaDetails;
static unsigned detailsGeneration;
static unsigned char *detailsRequested;
static int detailsCount,activeDetails,coverCursor,coverRows[COVER_LIMIT];
static HIMAGELIST covers;
static WCHAR *readMediaProperty(IPropertyStore *store,const PROPERTYKEY *key){
 PROPVARIANT v;PropVariantInit(&v);WCHAR *result=NULL;
 if(SUCCEEDED(IPropertyStore_GetValue(store,key,&v)) && v.vt!=VT_EMPTY)
  PropVariantToStringAlloc(&v,&result);
 PropVariantClear(&v);return result;
}
static void disposeDetails(MediaDetails *entry){
 CoTaskMemFree(entry->title);CoTaskMemFree(entry->artist);CoTaskMemFree(entry->album);
 if(entry->cover)DeleteObject(entry->cover);
 free(entry->path);free(entry);
}
static DWORD WINAPI readMediaDetails(void *arg){
 MediaDetails *entry=arg;
 HRESULT co=CoInitializeEx(NULL,COINIT_APARTMENTTHREADED);
 if(SUCCEEDED(co)){
  IPropertyStore *store=NULL;
  if(SUCCEEDED(SHGetPropertyStoreFromParsingName(entry->path,NULL,GPS_BESTEFFORT,&IID_IPropertyStore,(void**)&store))){
   entry->title=readMediaProperty(store,&PKEY_Title);
   entry->album=readMediaProperty(store,&PKEY_Music_AlbumTitle);
   entry->artist=readMediaProperty(store,&PKEY_Music_Artist);
   PROPVARIANT v;PropVariantInit(&v);
   if(SUCCEEDED(IPropertyStore_GetValue(store,&PKEY_Media_Duration,&v))&&v.vt==VT_UI8&&v.uhVal.QuadPart){
    unsigned long long seconds=v.uhVal.QuadPart/10000000;
    swprintf(entry->duration,32,L"%llu:%02llu",seconds/60,seconds%60);
   }
   PropVariantClear(&v);IPropertyStore_Release(store);
  }
  IShellItemImageFactory *images=NULL;
  if(SUCCEEDED(SHCreateItemFromParsingName(entry->path,NULL,&IID_IShellItemImageFactory,(void**)&images))){
   SIZE size={entry->edge,entry->edge};
   IShellItemImageFactory_GetImage(images,size,SIIGBF_THUMBNAILONLY,&entry->cover);
   IShellItemImageFactory_Release(images);
  }
  CoUninitialize();
 }
 if(!PostMessageW(entry->owner,MEDIA_DETAILS_READY,0,(LPARAM)entry))disposeDetails(entry);
 return 0;
}
static void clearDetails(void){
 ++detailsGeneration;free(detailsRequested);detailsRequested=NULL;detailsCount=0;
 ListView_SetImageList(mediaRows,NULL,LVSIL_SMALL);
 if(covers){ImageList_Destroy(covers);covers=NULL;}
 coverCursor=0;for(int i=0;i<COVER_LIMIT;i++)coverRows[i]=-1;
}
static void resetDetails(void){
 clearDetails();
 if(!mediaTheme() || browserPage==NAV_NOW)return;
 detailsCount=ListView_GetItemCount(mediaRows);
 detailsRequested=calloc(max(1,detailsCount),sizeof(*detailsRequested));
 int edge=scale(skin==XP?33:52);
 covers=ImageList_Create(edge,edge,ILC_COLOR32|ILC_MASK,1,8);
 if(!covers)return;
 SHSTOCKICONINFO stock={sizeof(stock)};
 HICON fallback=brandIcon;
 if(SUCCEEDED(SHGetStockIconInfo(SIID_MEDIACDAUDIO,SHGSI_ICON|SHGSI_LARGEICON,&stock)))fallback=stock.hIcon;
 ImageList_AddIcon(covers,fallback);if(stock.hIcon)DestroyIcon(stock.hIcon);
 ListView_SetImageList(mediaRows,covers,LVSIL_SMALL);
}
static void requestVisibleDetails(void){
 if(!detailsRequested||activeDetails>=2||!IsWindowVisible(mediaRows))return;
 int first=ListView_GetTopIndex(mediaRows);
 int end=min(detailsCount,first+ListView_GetCountPerPage(mediaRows)+2);
 for(int row=first;row<end&&activeDetails<2;row++){
  if(detailsRequested[row])continue;
  LVITEMW item={0};item.mask=LVIF_PARAM;item.iItem=row;
  if(!SendMessageW(mediaRows,LVM_GETITEMW,0,(LPARAM)&item))continue;
  const char *path=browserPage==NAV_LIBRARY?ap_library_item(player,(size_t)item.lParam):ap_item(player,(size_t)item.lParam);
  detailsRequested[row]=1;
  if(!path||strstr(path,"://"))continue;
  MediaDetails *entry=calloc(1,sizeof(*entry));
  if(!entry)continue;
  entry->path=wide(path);entry->owner=mainwin;entry->row=row;entry->generation=detailsGeneration;entry->edge=scale(skin==XP?33:52);
  if(entry->path && QueueUserWorkItem(readMediaDetails,entry,WT_EXECUTEDEFAULT))activeDetails++;
  else disposeDetails(entry);
 }
}
static void setRowImage(int row,int index){
 LVITEMW item={0};item.iItem=row;item.iImage=index;item.mask=LVIF_IMAGE;
 SendMessageW(mediaRows,LVM_SETITEMW,0,(LPARAM)&item);
}
static void attachCover(MediaDetails *entry){
 if(!covers || !entry->cover)return;
 BITMAP bitmap;
 if(!GetObjectW(entry->cover,sizeof(bitmap),&bitmap)||bitmap.bmWidth<=0||bitmap.bmHeight<=0)return;
 int edge=entry->edge,largest=max(bitmap.bmWidth,bitmap.bmHeight);
 int w=max(1,MulDiv(bitmap.bmWidth,edge,largest)),h=max(1,MulDiv(bitmap.bmHeight,edge,largest));
 // Shell handlers can return non-square bitmaps. Fit each within one image-list cell.
 HDC base=GetDC(mediaRows),src=CreateCompatibleDC(base),dst=CreateCompatibleDC(base);
 HBITMAP square=CreateCompatibleBitmap(base,edge,edge);ReleaseDC(mediaRows,base);
 if(square&&src&&dst){
  HGDIOBJ os=SelectObject(src,entry->cover),od=SelectObject(dst,square);
  RECT bounds={0,0,edge,edge};fillColor(dst,bounds,ListView_GetBkColor(mediaRows));
  SetStretchBltMode(dst,HALFTONE);SetBrushOrgEx(dst,0,0,NULL);
  StretchBlt(dst,(edge-w)/2,(edge-h)/2,w,h,src,0,0,bitmap.bmWidth,bitmap.bmHeight,SRCCOPY);
  SelectObject(src,os);SelectObject(dst,od);
  int slot=coverCursor%COVER_LIMIT,index=slot+1,previous=coverRows[slot];
  bool ok=previous<0?ImageList_Add(covers,square,NULL)==index:ImageList_Replace(covers,index,square,NULL);
  if(ok){
   if(previous>=0){setRowImage(previous,0);detailsRequested[previous]=0;}
   setRowImage(entry->row,index);coverRows[slot]=entry->row;coverCursor++;
  }
 }
 if(square)DeleteObject(square);if(src)DeleteDC(src);if(dst)DeleteDC(dst);
}
static void receiveMediaDetails(MediaDetails *entry){
 --activeDetails;
 if(entry->generation==detailsGeneration && entry->row<detailsCount){
  WCHAR *values[]={entry->title,entry->album,entry->artist,entry->duration};
  for(int column=0;column<4;column++)if(values[column]&&*values[column]){
   LVITEMW item={0};item.iSubItem=column;item.pszText=values[column];
   SendMessageW(mediaRows,LVM_SETITEMTEXTW,entry->row,(LPARAM)&item);
  }
  attachCover(entry);
 }
 disposeDetails(entry);
}
