// PlayerApp's Windows Media Player-style navigation. Every row refers to a queue or library index.
-(BOOL)hasMediaLayout {return skin==SEVEN || skin==XP;}
-(NSButton *)navigationButton:(NSString *)title action:(SEL)action {
 NSButton *button=[NSButton buttonWithTitle:title target:self action:action];
 button.bordered=NO;button.alignment=NSTextAlignmentLeft;
 button.font=[NSFont systemFontOfSize:13];button.focusRingType=NSFocusRingTypeExterior;
 return button;
}
-(void)buildMediaLayout:(NSView *)root {
 searchNames=[NSMutableDictionary new];metadataCache=[NSCache new];metadataCache.countLimit=512;metadataPending=[NSMutableSet new];metadataWorkers=[NSOperationQueue new];metadataWorkers.maxConcurrentOperationCount=2;
 transportCapsule=[MediaPane new];transportCapsule.style=MediaSevenHeader;transportCapsule.wantsLayer=YES;transportCapsule.layer.cornerRadius=15;
 transportCapsule.layer.borderColor=rgb(0x91a7b9).CGColor;transportCapsule.layer.borderWidth=1;
 [root addSubview:transportCapsule positioned:NSWindowBelow relativeTo:bottom];transportCapsule.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[transportCapsule.leadingAnchor constraintEqualToAnchor:bottom.leadingAnchor constant:-8],[transportCapsule.trailingAnchor constraintEqualToAnchor:bottom.trailingAnchor constant:8],[transportCapsule.topAnchor constraintEqualToAnchor:bottom.topAnchor constant:2],[transportCapsule.bottomAnchor constraintEqualToAnchor:bottom.bottomAnchor constant:-2]]];
 navPane=[MediaPane new];mediaHeader=[MediaPane new];browserPane=[MediaPane new];
 for(NSView *view in @[navPane,mediaHeader,browserPane]){[root addSubview:view];view.translatesAutoresizingMaskIntoConstraints=NO;}
 [NSLayoutConstraint activateConstraints:@[
  [mediaHeader.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:3],[mediaHeader.trailingAnchor constraintEqualToAnchor:root.trailingAnchor constant:-3],
  [mediaHeader.topAnchor constraintEqualToAnchor:root.topAnchor constant:3],[mediaHeader.heightAnchor constraintEqualToConstant:42],
  [navPane.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:3],[navPane.topAnchor constraintEqualToAnchor:mediaHeader.bottomAnchor],
  [navPane.bottomAnchor constraintEqualToAnchor:timeline.topAnchor constant:-5],[navPane.widthAnchor constraintEqualToConstant:132],
  [browserPane.trailingAnchor constraintEqualToAnchor:root.trailingAnchor constant:-3],[browserPane.topAnchor constraintEqualToAnchor:mediaHeader.bottomAnchor],
  [browserPane.bottomAnchor constraintEqualToAnchor:navPane.bottomAnchor]
 ]];
 browserLeading=[browserPane.leadingAnchor constraintEqualToAnchor:navPane.trailingAnchor];
 browserWidth=[browserPane.widthAnchor constraintEqualToConstant:210];browserLeading.active=YES;
 crumb=label(@"",14,YES);crumb.lineBreakMode=NSLineBreakByTruncatingMiddle;
 [crumb setContentCompressionResistancePriority:200 forOrientation:NSLayoutConstraintOrientationHorizontal];
 searchField=[NSSearchField new];searchField.placeholderString=@"Search";searchField.delegate=self;
 [searchField.widthAnchor constraintEqualToConstant:158].active=YES;
 headerOpen=[self navigationButton:@"Open files…" action:@selector(open:)];
 NSStackView *header=[self row:@[crumb,[NSView new],searchField,headerOpen]];
 [mediaHeader addSubview:header];header.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[header.leadingAnchor constraintEqualToAnchor:mediaHeader.leadingAnchor constant:12],
  [header.trailingAnchor constraintEqualToAnchor:mediaHeader.trailingAnchor constant:-12],[header.centerYAnchor constraintEqualToAnchor:mediaHeader.centerYAnchor]]];
 NSMutableArray *items=[NSMutableArray new];
 for(NSString *title in @[@"Now Playing",@"Library",@"Current Playlist"]){
  NSButton *button=[self navigationButton:title action:@selector(navigateMedia:)];button.tag=items.count;
  NSString *symbol=@[@"play.rectangle",@"music.note.house",@"list.bullet"][button.tag];
  button.image=[NSImage imageWithSystemSymbolName:symbol accessibilityDescription:nil];button.imagePosition=NSImageLeading;
  [items addObject:button];
 }
 navButtons=[items copy];
 NSBox *divider=[NSBox new];divider.boxType=NSBoxSeparator;[items addObject:divider];
 [items addObject:[self navigationButton:@"Open playlist…" action:@selector(openPlaylist:)]];
 [items addObject:[self navigationButton:@"Save playlist…" action:@selector(savePlaylist:)]];
 NSStackView *nav=[NSStackView stackViewWithViews:items];nav.orientation=NSUserInterfaceLayoutOrientationVertical;
 nav.alignment=NSLayoutAttributeLeading;nav.spacing=7;
 for(NSView *v in items){[v.widthAnchor constraintEqualToConstant:112].active=YES;
  if([v isKindOfClass:NSButton.class])[v.heightAnchor constraintEqualToConstant:28].active=YES;}
 [navPane addSubview:nav];nav.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[nav.leadingAnchor constraintEqualToAnchor:navPane.leadingAnchor constant:10],[nav.topAnchor constraintEqualToAnchor:navPane.topAnchor constant:15]]];
 trackTable=[NSTableView new];trackTable.delegate=self;trackTable.dataSource=self;trackTable.target=self;
 trackTable.doubleAction=@selector(playTableRow:);trackTable.allowsColumnReordering=NO;
 trackTable.columnAutoresizingStyle=NSTableViewUniformColumnAutoresizingStyle;
 for(NSString *key in @[@"Cover",@"Title",@"Album",@"Artist",@"Length"]){
  NSTableColumn *column=[[NSTableColumn alloc]initWithIdentifier:key];column.title=[key isEqual:@"Cover"]?@"":key;
  column.width=[key isEqual:@"Title"]?205:[key isEqual:@"Cover"]?52:[key isEqual:@"Length"]?58:125;column.minWidth=44;
  if([key isEqual:@"Cover"]||[key isEqual:@"Length"])column.resizingMask=NSTableColumnNoResizing;
  [trackTable addTableColumn:column];
 }
 trackScroll=[NSScrollView new];trackScroll.documentView=trackTable;trackScroll.hasVerticalScroller=YES;trackScroll.autohidesScrollers=YES;
 [browserPane addSubview:trackScroll];trackScroll.translatesAutoresizingMaskIntoConstraints=NO;
 tableHeading=label(@"",12,YES);[browserPane addSubview:tableHeading];tableHeading.translatesAutoresizingMaskIntoConstraints=NO;
 queueArt=[NSImageView new];queueArt.imageScaling=NSImageScaleProportionallyUpOrDown;
 queueArt.contentTintColor=rgb(0xbac7e8);[browserPane addSubview:queueArt];queueArt.translatesAutoresizingMaskIntoConstraints=NO;
 artHeight=[queueArt.heightAnchor constraintEqualToConstant:0];artHeight.active=YES;
 [NSLayoutConstraint activateConstraints:@[[queueArt.topAnchor constraintEqualToAnchor:tableHeading.bottomAnchor constant:3],[queueArt.centerXAnchor constraintEqualToAnchor:browserPane.centerXAnchor],[queueArt.widthAnchor constraintEqualToConstant:100],
  [tableHeading.topAnchor constraintEqualToAnchor:browserPane.topAnchor constant:7],[tableHeading.leadingAnchor constraintEqualToAnchor:browserPane.leadingAnchor constant:8],
  [trackScroll.topAnchor constraintEqualToAnchor:queueArt.bottomAnchor constant:4],[trackScroll.leadingAnchor constraintEqualToAnchor:browserPane.leadingAnchor],
  [trackScroll.trailingAnchor constraintEqualToAnchor:browserPane.trailingAnchor],[trackScroll.bottomAnchor constraintEqualToAnchor:browserPane.bottomAnchor]]];
}
-(void)layoutMediaChrome {
 BOOL active=[self hasMediaLayout]&&!inFull&&!minimalInterface&&!wallpaper, playing=mediaPage==0, sidebar=skin==XP && playing;
 navPane.hidden=mediaHeader.hidden=!active;browserPane.hidden=!active || (playing&&!sidebar);
 transportCapsule.hidden=!(active&&skin==SEVEN);
 screen.hidden=active && !playing;
 screenLeft.constant=active?135:0;screenRight.constant=active && sidebar?-213:0;
 mediaTop.active=active;searchField.hidden=playing;
 if(!active)return;
 if(sidebar){browserLeading.active=NO;browserWidth.active=YES;}else{browserWidth.active=NO;browserLeading.active=YES;}
 navPane.style=skin==XP?MediaXPNav:MediaSevenNav;
 mediaHeader.style=skin==XP?MediaXPHeader:MediaSevenHeader;
 browserPane.style=sidebar?MediaXPQueue:MediaSevenNav;
 crumb.textColor=skin==XP?NSColor.whiteColor:rgb(0x183b60);
 headerOpen.attributedTitle=[[NSAttributedString alloc]initWithString:@"Open files…" attributes:@{NSForegroundColorAttributeName:crumb.textColor,NSFontAttributeName:headerOpen.font}];
 crumb.stringValue=skin==XP?@[@"RMP  ›  Now Playing",@"Media Library",@"Current Playlist"][mediaPage]:
  @[@"Now Playing",@"Library  ›  Media  ›  All media",@"Library  ›  Playlists  ›  Current playlist"][mediaPage];
 for(NSButton *b in navButtons){
  BOOL chosen=b.tag==mediaPage;b.wantsLayer=YES;b.layer.cornerRadius=3;
  b.layer.backgroundColor=chosen?rgb(skin==XP?0xb1c3e6:0xc4def4).CGColor:NSColor.clearColor.CGColor;
  b.font=[NSFont systemFontOfSize:13 weight:chosen?NSFontWeightSemibold:NSFontWeightRegular];
 }
 trackTable.intercellSpacing=NSMakeSize(3,1);trackTable.rowHeight=sidebar?25:54;
 trackTable.backgroundColor=sidebar?rgb(0x38436a):rgb(0xeff5fb);
 trackTable.usesAlternatingRowBackgroundColors=NO;trackTable.gridStyleMask=sidebar?NSTableViewGridNone:NSTableViewSolidVerticalGridLineMask;trackTable.gridColor=rgb(0xdbE5f0);
 for(NSTableColumn *c in trackTable.tableColumns)c.hidden=sidebar && ![c.identifier isEqual:@"Title"];

 tableHeading.textColor=sidebar?NSColor.whiteColor:rgb(0x234a71);
 artHeight.constant=sidebar?98:0;queueArt.hidden=!sidebar;
 tableHeading.stringValue=mediaPage==1?@"All media":@"Current playlist";
 [self rebuildMediaRows];
}
-(void)rebuildMediaRows {
 if(!p || ![self hasMediaLayout] || browserPane.hidden)return;
 BOOL library=mediaPage==1;
 size_t count=library?ap_library_count(p):ap_count(p);
 NSString *query=searchField.hidden?@"":searchField.stringValue;
 NSMutableArray *indices=[NSMutableArray arrayWithCapacity:count];
 for(size_t i=0;i<count;i++){
  const char *raw=library?ap_library_item(p,i):ap_item(p,i);
  if(!raw)continue;
   NSString *source=[NSString stringWithUTF8String:raw];
  if(!query.length || [source localizedCaseInsensitiveContainsString:query] || [searchNames[source] localizedCaseInsensitiveContainsString:query])[indices addObject:@(i)];
 }
 if(!queueArt.hidden){
  NSInteger current=ap_index(p);const char *entry=count?ap_item(p,current<0?0:(NSUInteger)current):NULL;
  queueArt.image=entry?[self metadataForPath:[NSString stringWithUTF8String:entry]][@"Cover"]:nil;
  if(!queueArt.image)queueArt.image=[NSImage imageWithSystemSymbolName:@"music.note.tv" accessibilityDescription:@"Now playing"];
 }
 mediaRows=indices;[trackTable reloadData];[trackTable sizeToFit];
 if(query.length)[self indexSearchBatch];
}
-(void)indexSearchBatch {
 if(!p || browserPane.hidden || searchField.hidden || !searchField.stringValue.length)return;
 BOOL library=mediaPage==1;NSUInteger count=library?ap_library_count(p):ap_count(p);
 for(unsigned batch=0;batch<64 && searchCursor<count && metadataPending.count<12;batch++){
  NSUInteger idx=searchCursor++;const char *raw=library?ap_library_item(p,idx):ap_item(p,idx);
  if(!raw)continue;
  NSString *path=[NSString stringWithUTF8String:raw];
  if(!searchNames[path])[self metadataForPath:path];
 }
 if(searchCursor<count && metadataPending.count<12)
  [self performSelector:@selector(indexSearchBatch) withObject:nil afterDelay:0.02];
}
-(void)refilterMetadata {refillScheduled=NO;[self rebuildMediaRows];}
-(NSInteger)numberOfRowsInTableView:(NSTableView *)table{return table==queueList?(p?ap_count(p):0):mediaRows.count;}
-(NSDictionary *)metadataForPath:(NSString *)path {
 NSDictionary *cached=[metadataCache objectForKey:path];
 if(cached || [metadataPending containsObject:path] || !path.isAbsolutePath)return cached;
 [metadataPending addObject:path];
 [metadataWorkers addOperationWithBlock:^{@autoreleasepool{
  AVURLAsset *asset=[AVURLAsset URLAssetWithURL:[NSURL fileURLWithPath:path] options:@{AVURLAssetPreferPreciseDurationAndTimingKey:@NO}];
  NSMutableDictionary *values=[NSMutableDictionary new];
  for(AVMetadataItem *item in asset.commonMetadata){
   if([item.commonKey isEqual:AVMetadataCommonKeyTitle] && item.stringValue.length)values[@"Title"]=item.stringValue;
   if([item.commonKey isEqual:AVMetadataCommonKeyAlbumName] && item.stringValue.length)values[@"Album"]=item.stringValue;
   if([item.commonKey isEqual:AVMetadataCommonKeyArtist] && item.stringValue.length)values[@"Artist"]=item.stringValue;
   if([item.commonKey isEqual:AVMetadataCommonKeyArtwork] && item.dataValue.length){
    NSImage *image=[[NSImage alloc]initWithData:item.dataValue];if(image)values[@"Cover"]=image;
   }
  }
  CMTime time=asset.duration;if(CMTIME_IS_NUMERIC(time)&&time.timescale>0)values[@"Length"]=stamp((int64_t)llround(1000.0*time.value/time.timescale));
  [[NSOperationQueue mainQueue]addOperationWithBlock:^{
   self->searchNames[path]=[NSString stringWithFormat:@"%@ %@ %@",values[@"Title"]?:@"",values[@"Album"]?:@"",values[@"Artist"]?:@""];
   if(self->searchField.stringValue.length && !self->refillScheduled){
    self->refillScheduled=YES;[self performSelector:@selector(refilterMetadata) withObject:nil afterDelay:0.08];
   }
   [self->metadataPending removeObject:path];[self->metadataCache setObject:values forKey:path];
   if(!self->queueArt.hidden && self->p && ap_count(self->p) && values[@"Cover"]){
    NSInteger current=ap_index(self->p);const char *entry=ap_item(self->p,current<0?0:(NSUInteger)current);
    if(entry && [path isEqualToString:[NSString stringWithUTF8String:entry]])self->queueArt.image=values[@"Cover"];
   }
   if(!self->browserPane.hidden){
    NSRange range=[self->trackTable rowsInRect:self->trackTable.visibleRect];
    if(range.location!=NSNotFound && range.length)[self->trackTable reloadDataForRowIndexes:[NSIndexSet indexSetWithIndexesInRange:range] columnIndexes:[NSIndexSet indexSetWithIndexesInRange:NSMakeRange(0,self->trackTable.numberOfColumns)]];
   }
  }];
 }}];
 return nil;
}
-(NSView *)tableView:(NSTableView *)table viewForTableColumn:(NSTableColumn *)column row:(NSInteger)row {
 if(table==queueList){
  if(!p||row<0||row>=ap_count(p))return nil;
  NSTextField *text=[table makeViewWithIdentifier:@"queue" owner:self];if(!text){text=label(@"",12,NO);text.identifier=@"queue";}text.stringValue=[self display:row];return text;
 }
 if(row<0 || row>=mediaRows.count || !p)return nil;
 size_t i=mediaRows[row].unsignedIntegerValue;
 const char *raw=mediaPage==1?ap_library_item(p,i):ap_item(p,i);if(!raw)return nil;
 NSString *path=[NSString stringWithUTF8String:raw];
 NSDictionary *info=[self metadataForPath:path];
 if([column.identifier isEqual:@"Cover"]){
  NSImageView *view=[table makeViewWithIdentifier:@"Cover" owner:self];
  if(!view){view=[NSImageView new];view.identifier=@"Cover";view.imageScaling=NSImageScaleProportionallyDown;}
  view.image=info[@"Cover"]?:[NSImage imageWithSystemSymbolName:@"music.note" accessibilityDescription:@"Media"];
  view.contentTintColor=rgb(0x6892b6);return view;
 }
 NSString *value=info[column.identifier];
 if(!value && [column.identifier isEqual:@"Title"])value=path.lastPathComponent.stringByRemovingPercentEncoding?:path.lastPathComponent;
 NSTextField *field=[table makeViewWithIdentifier:column.identifier owner:self];
 if(!field){field=label(@"",12,NO);field.identifier=column.identifier;}
 field.stringValue=value?:@"";field.toolTip=path;
 field.textColor=(skin==XP && mediaPage==0)?NSColor.whiteColor:rgb(0x192f48);
 return field;
}
-(void)controlTextDidChange:(NSNotification *)note{if(note.object==searchField){searchCursor=0;[self rebuildMediaRows];}}
-(void)navigateMedia:(NSButton *)b{pendingVideoSwitch=NO;searchCursor=0;mediaPage=b.tag;searchField.stringValue=@"";[self arrangeChrome];}
-(void)playTableRow:(id)sender {
 NSInteger row=trackTable.clickedRow;if(row<0)row=trackTable.selectedRow;
 if(row<0 || row>=mediaRows.count)return;
 NSUInteger i=mediaRows[row].unsignedIntegerValue;
 if(mediaPage==1){if(ap_library_play(p,i))[self showPlayingItem:(NSUInteger)ap_index(p)];else NSBeep();}
 else [self launch:i];
}
-(void)showPlayingItem:(NSUInteger)i {
 if(i>=ap_count(p))return;
 previous=-1;welcome.hidden=YES;cone.hidden=YES;name.stringValue=[self display:i];
 window.title=[@"RMP — " stringByAppendingString:name.stringValue];
 pendingVideoSwitch=skin==SEVEN && mediaPage!=0;
 if([self hasMediaLayout])[self rebuildMediaRows];
}
-(void)playlistError:(NSString *)message {
 NSAlert *alert=[NSAlert new];alert.messageText=message;[alert runModal];
}
-(void)savePlaylist:(id)sender {
 if(!ap_count(p)){[self playlistError:@"Add media to the queue before saving a playlist."];return;}
 NSSavePanel *savePanel=[NSSavePanel savePanel];savePanel.allowedFileTypes=@[@"m3u8",@"m3u"];savePanel.nameFieldStringValue=@"Playlist.m3u8";
 if([savePanel runModal]==NSModalResponseOK && !ap_playlist_save(p,savePanel.URL.fileSystemRepresentation))[self playlistError:@"RMP could not write this playlist."];
}
-(void)openPlaylist:(id)sender {
 NSOpenPanel *picker=[NSOpenPanel openPanel];picker.allowedFileTypes=@[@"m3u8",@"m3u"];
 if([picker runModal]!=NSModalResponseOK)return;
 if(!ap_playlist_load(p,picker.URL.fileSystemRepresentation)){[self playlistError:@"RMP could not read this playlist."];return;}
 searchField.stringValue=@"";
 if([self hasMediaLayout]){mediaPage=2;[self arrangeChrome];}
 if(ap_count(p))[self launch:0];else{window.title=@"RMP";name.stringValue=@"RMP";}
}
