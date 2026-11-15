// Desktop shortcut: a symlink to the installed app, on by default. Turned off in Tools > Desktop Shortcut.
static BOOL desktopShortcutWanted(void){
 id v=CFBridgingRelease(CFPreferencesCopyAppValue(CFSTR("DesktopShortcut"),CFSTR("io.github.kmthegamer2022.rmp")));
 return v?[v boolValue]:YES;
}
static void setDesktopShortcut(BOOL on,NSString *app){
 CFPreferencesSetAppValue(CFSTR("DesktopShortcut"),on?kCFBooleanTrue:kCFBooleanFalse,CFSTR("io.github.kmthegamer2022.rmp"));
 CFPreferencesAppSynchronize(CFSTR("io.github.kmthegamer2022.rmp"));
 NSFileManager *fm=NSFileManager.defaultManager;
 NSString *link=[NSHomeDirectory() stringByAppendingPathComponent:@"Desktop/RMP.app"];
 NSString *old=[fm destinationOfSymbolicLinkAtPath:link error:nil];
 if(old)[fm removeItemAtPath:link error:nil];
 else if([fm fileExistsAtPath:link])return;
 if(on && app.length)[fm createSymbolicLinkAtPath:link withDestinationPath:app error:nil];
}
// Native AppKit menus. Engine-backed submenus are populated when opened so a
// track, title or device appearing after playback starts is immediately usable.
-(NSMenuItem *)command:(NSString *)title in:(NSMenu *)menu key:(NSString *)key value:(NSString *)value {
 NSMenuItem *item=[self item:menu title:title action:@selector(runMenuCommand:) key:@""];
 item.representedObject=@[key,value?:@""];return item;
}
-(NSMenu *)options:(NSString *)title in:(NSMenu *)parent key:(NSString *)key {
 NSMenu *m=[self menu:title parent:parent];[optionKeys setObject:key forKey:m];m.delegate=self;return m;
}
-(void)installPlayerMenus:(NSMenu *)bar {
 optionKeys=[NSMapTable strongToStrongObjectsMapTable];
 NSMenu *m=[self menu:@"Media" parent:bar];
 [self item:m title:@"Open File…" action:@selector(openSingle:) key:@"o"];
 [self item:m title:@"Open Multiple Files…" action:@selector(open:) key:@"O"];
 [self command:@"Open Folder…" in:m key:@"folder" value:nil];
 [self command:@"Open Disc…" in:m key:@"disc" value:nil];
 [self item:m title:@"Open Network Stream…" action:@selector(stream:) key:@"u"];
 [self command:@"Open Capture Device…" in:m key:@"capture" value:nil];
 [self command:@"Open Location from Clipboard" in:m key:@"clipboard" value:nil];
 recentMenu=[self menu:@"Open Recent Media" parent:m];recentMenu.delegate=self;
 [m addItem:NSMenuItem.separatorItem];
 [self item:m title:@"Open Playlist…" action:@selector(openPlaylist:) key:@""];
 [self item:m title:@"Save Playlist to File…" action:@selector(savePlaylist:) key:@""];
 [self command:@"Convert / Save…" in:m key:@"convert" value:nil];
 [self command:@"Stream…" in:m key:@"stream-out" value:nil];
 [self command:@"Stop Media Output" in:m key:@"output-stop" value:nil];
 [m addItem:NSMenuItem.separatorItem];
 [self command:@"Quit at the End of Playlist" in:m key:@"quit-end" value:nil];
 [m addItemWithTitle:@"Quit RMP" action:@selector(terminate:) keyEquivalent:@""];
 m=[self menu:@"Playback" parent:bar];
 [self options:@"Title" in:m key:@"title"];[self options:@"Chapter" in:m key:@"chapter"];
 [self options:@"Programme" in:m key:@"programme"];[self options:@"Custom Bookmarks" in:m key:@"bookmark"];
 [m addItem:NSMenuItem.separatorItem];[self options:@"Renderer" in:m key:@"renderer"];
 [self options:@"Speed" in:m key:@"speed"];[m addItem:NSMenuItem.separatorItem];
 [self command:@"Jump Forward" in:m key:@"jump" value:@"10"];[self command:@"Jump Backward" in:m key:@"jump" value:@"-10"];
 [self command:@"Jump to Specific Time…" in:m key:@"time" value:nil].keyEquivalent=@"t";
 [m addItem:NSMenuItem.separatorItem];playMenuItem=[self item:m title:@"Play / Pause" action:@selector(toggle:) key:@" "];playMenuItem.keyEquivalentModifierMask=0;
 [self item:m title:@"Stop" action:@selector(stopPlayback:) key:@""];
 [self item:m title:@"Previous" action:@selector(prev:) key:@"["];[self item:m title:@"Next" action:@selector(next:) key:@"]"];
 [self command:@"Record" in:m key:@"record" value:nil];
 m=[self menu:@"Audio" parent:bar];audio=[self options:@"Audio Track" in:m key:@"audio-track"];[self options:@"Audio Device" in:m key:@"audio-device"];
 [self options:@"Stereo Mode" in:m key:@"stereo"];[self options:@"Visualizations" in:m key:@"visualization"];[m addItem:NSMenuItem.separatorItem];
 [self command:@"Increase Volume" in:m key:@"volume-step" value:@"5"];[self command:@"Decrease Volume" in:m key:@"volume-step" value:@"-5"];[self command:@"Mute" in:m key:@"mute" value:nil];
 m=[self menu:@"Video" parent:bar];[self options:@"Video Track" in:m key:@"video-track"];[m addItem:NSMenuItem.separatorItem];
 NSMenuItem *full=[self item:m title:@"Fullscreen" action:@selector(full:) key:@"f"];full.keyEquivalentModifierMask=NSEventModifierFlagControl|NSEventModifierFlagCommand;
 [self command:@"Always Fit Window" in:m key:@"fit" value:nil];[self command:@"Set as Wallpaper" in:m key:@"wallpaper" value:nil];
 [self options:@"Zoom" in:m key:@"zoom"];[self options:@"Aspect Ratio" in:m key:@"aspect"];[self options:@"Crop" in:m key:@"crop"];
 [self options:@"Deinterlace" in:m key:@"deinterlace"];[self options:@"Deinterlace Mode" in:m key:@"deinterlace-mode"];
 [m addItem:NSMenuItem.separatorItem];[self command:@"Take Snapshot…" in:m key:@"snapshot" value:nil];
 m=[self menu:@"Subtitle" parent:bar];[self command:@"Add Subtitle File…" in:m key:@"subtitle-file" value:nil];subs=[self options:@"Sub Track" in:m key:@"subtitle-track"];
 m=[self menu:@"Tools" parent:bar];[self command:@"Effects and Filters…" in:m key:@"effects" value:nil].keyEquivalent=@"e";
 [self command:@"Track Synchronization…" in:m key:@"sync" value:nil];
 for(NSArray *pair in @[@[@"Media Information",@"media-info"],@[@"Codec Information",@"codec-info"],@[@"VLM Configuration",@"vlm"],@[@"Program Guide",@"guide"],@[@"Messages",@"messages"],@[@"Plugins and Extensions",@"plugins"]])
  [self command:pair[0] in:m key:pair[1] value:nil];
 [m addItem:NSMenuItem.separatorItem];[self command:@"Customize Interface…" in:m key:@"customize" value:nil];[self command:@"Preferences…" in:m key:@"preferences" value:nil];[self command:@"Desktop Shortcut" in:m key:@"desktop-shortcut" value:nil].state=desktopShortcutWanted()?NSControlStateValueOn:NSControlStateValueOff;
 m=[self menu:@"View" parent:bar];[self command:@"Playlist" in:m key:@"playlist" value:nil].keyEquivalent=@"l";
 [self command:@"Docked Playlist" in:m key:@"docked" value:nil];[self command:@"Always on Top" in:m key:@"top" value:nil];
 [m addItem:NSMenuItem.separatorItem];[self command:@"Minimal Interface" in:m key:@"minimal" value:nil];
 [self item:m title:@"Fullscreen Interface" action:@selector(full:) key:@""];
 [self command:@"Advanced Controls" in:m key:@"advanced" value:nil];[self command:@"Status Bar" in:m key:@"status" value:nil];
 NSMenu *interfaces=[self menu:@"Add Interface" parent:m];[self command:@"RMP Terminal" in:interfaces key:@"terminal" value:nil];
 [self command:@"VLsub…" in:m key:@"vlsub" value:nil];
 queue=[NSMenu new];queue.delegate=self;
 m=[self menu:@"Help" parent:bar];[self command:@"Help" in:m key:@"help" value:nil];
 [self item:m title:@"Check for Updates…" action:@selector(manualUpdate:) key:@""];
 [m addItem:NSMenuItem.separatorItem];[m addItemWithTitle:@"About RMP" action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""];
}
-(void)menuNote:(NSString *)title detail:(NSString *)message {
 NSAlert *alert=[NSAlert new];alert.messageText=title;
 if(message.length>360 || [message componentsSeparatedByString:@"\n"].count>7){
  NSScrollView *scroll=[[NSScrollView alloc]initWithFrame:NSMakeRect(0,0,475,255)];scroll.hasVerticalScroller=YES;
  NSTextView *text=[[NSTextView alloc]initWithFrame:scroll.bounds];text.editable=NO;text.font=[NSFont systemFontOfSize:12];text.string=message;scroll.documentView=text;alert.accessoryView=scroll;
 }else alert.informativeText=message?:@"";
 [alert runModal];
}
-(NSString *)askValue:(NSString *)title detail:(NSString *)detail initial:(NSString *)initial {
 NSAlert *a=[NSAlert new];a.messageText=title;a.informativeText=detail?:@"";
 NSTextField *field=[[NSTextField alloc]initWithFrame:NSMakeRect(0,0,342,24)];field.stringValue=initial?:@"";
 a.accessoryView=field;a.window.initialFirstResponder=field;[a addButtonWithTitle:@"OK"];[a addButtonWithTitle:@"Cancel"];
 return [a runModal]==NSAlertFirstButtonReturn?field.stringValue:nil;
}
-(NSString *)engineAction:(NSString *)key value:(NSString *)value {
 return takeTerminalString(ap_menu_action(p,key.UTF8String,(value?:@"").UTF8String));
}
-(void)applyOption:(NSMenuItem *)entry {
 NSArray *pair=entry.representedObject;
 NSString *result=[self engineAction:pair[0] value:pair[1]];
 if(result.length)[self menuNote:entry.title detail:result];
}
-(BOOL)populateMediaMenu:(NSMenu *)m {
 if(m==recentMenu){
  [m removeAllItems];NSArray *recent=[NSUserDefaults.standardUserDefaults arrayForKey:@"RecentMedia"];
  for(NSString *path in recent){NSMenuItem *entry=[self command:path.lastPathComponent.length?path.lastPathComponent:path in:m key:@"recent" value:path];entry.toolTip=path;}
  if(!recent.count)[m addItemWithTitle:@"No recent media" action:nil keyEquivalent:@""];
  else{[m addItem:NSMenuItem.separatorItem];[self command:@"Clear Recent Media" in:m key:@"clear-recent" value:nil];}return YES;
 }
 NSString *key=[optionKeys objectForKey:m];if(!key)return NO;[m removeAllItems];
 NSString *rows=p?takeTerminalString(ap_menu_options(p,key.UTF8String)):@"";
 for(NSString *line in [rows componentsSeparatedByString:@"\n"]){
  NSRange tab=[line rangeOfString:@"\t"];if(tab.location==NSNotFound)continue;
  NSString *ident=[line substringToIndex:tab.location],*title=[line substringFromIndex:NSMaxRange(tab)];
  NSMenuItem *entry=[self item:m title:title action:@selector(applyOption:) key:@""];entry.representedObject=@[key,ident];
 }
 if(!m.numberOfItems)[m addItemWithTitle:@"None available" action:nil keyEquivalent:@""];
 return YES;
}
-(void)rememberRecent:(NSString *)path{
 if(!path.length)return;
 NSMutableArray *recent=[([NSUserDefaults.standardUserDefaults arrayForKey:@"RecentMedia"]?:@[]) mutableCopy];
 [recent removeObject:path];[recent insertObject:path atIndex:0];if(recent.count>20)[recent removeLastObject];
 [NSUserDefaults.standardUserDefaults setObject:recent forKey:@"RecentMedia"];
}
-(void)openSingle:(id)sender{
 ap_prepare(p);NSOpenPanel *picker=[NSOpenPanel openPanel];if([picker runModal]==NSModalResponseOK)[self accept:picker.URLs];
}
-(void)openFolder {
 NSOpenPanel *picker=[NSOpenPanel openPanel];picker.canChooseDirectories=YES;picker.canChooseFiles=NO;
 if([picker runModal]!=NSModalResponseOK)return;
 NSURL *root=picker.URL;ap_prepare(p);
 NSArray *declared=[NSBundle.mainBundle objectForInfoDictionaryKey:@"CFBundleDocumentTypes"];
 NSMutableSet *extensions=[NSMutableSet new];for(NSDictionary *type in declared)[extensions addObjectsFromArray:type[@"CFBundleTypeExtensions"]?:@[]];
 dispatch_async(dispatch_get_global_queue(QOS_CLASS_USER_INITIATED,0),^{
  NSMutableArray *files=[NSMutableArray new];
  for(NSURL *file in [NSFileManager.defaultManager enumeratorAtURL:root includingPropertiesForKeys:@[NSURLIsRegularFileKey] options:NSDirectoryEnumerationSkipsHiddenFiles|NSDirectoryEnumerationSkipsPackageDescendants errorHandler:nil]){
   if(![extensions containsObject:file.pathExtension.lowercaseString])continue;
   NSNumber *regular=nil;[file getResourceValue:&regular forKey:NSURLIsRegularFileKey error:nil];if(regular.boolValue)[files addObject:file];
  }
  [files sortUsingComparator:^NSComparisonResult(NSURL *a,NSURL *b){return [a.path localizedStandardCompare:b.path];}];
  dispatch_async(dispatch_get_main_queue(),^{if(files.count)[self accept:files];else[self menuNote:@"Open Folder" detail:@"No supported media files were found in this folder."];});
 });
}
-(void)openLocation:(NSString *)location{
 if(!location.length)return;
 NSString *path=location.stringByExpandingTildeInPath;
 if([NSFileManager.defaultManager fileExistsAtPath:path]){[self accept:@[[NSURL fileURLWithPath:path]]];return;}
 NSURL *url=[NSURL URLWithString:location];if(url.scheme.length){[self accept:@[url]];return;}
 [self menuNote:@"Open Location" detail:@"Choose a media file or enter a URL including its protocol."];
}
-(void)pickOutput:(NSString *)key title:(NSString *)title extension:(NSString *)ext {
 NSSavePanel *picker=[NSSavePanel savePanel];picker.title=title;picker.allowedFileTypes=@[ext];
 NSString *name=ap_index(p)>=0?[self display:(size_t)ap_index(p)]:@"RMP";
 picker.nameFieldStringValue=[name.stringByDeletingPathExtension stringByAppendingPathExtension:ext];
 if([picker runModal]!=NSModalResponseOK)return;
 NSString *result=[self engineAction:key value:picker.URL.path];
 if(result.length)[self menuNote:title detail:result];
 else if([key isEqual:@"record"])recording=YES;
}
-(void)openCapture {
 NSAlert *a=[NSAlert new];a.messageText=@"Open Capture Device";a.informativeText=@"Choose the source to play.";
 NSPopUpButton *devices=[[NSPopUpButton alloc]initWithFrame:NSMakeRect(0,0,320,28) pullsDown:NO];
 [devices addItemWithTitle:@"Screen"];devices.lastItem.representedObject=@"screen://";
 for(AVCaptureDevice *d in [AVCaptureDevice devicesWithMediaType:AVMediaTypeVideo]){
  [devices addItemWithTitle:d.localizedName];devices.lastItem.representedObject=[@"avcapture://" stringByAppendingString:d.uniqueID];}
 for(AVCaptureDevice *d in [AVCaptureDevice devicesWithMediaType:AVMediaTypeAudio]){
  [devices addItemWithTitle:d.localizedName];devices.lastItem.representedObject=[@"avaudiocapture://" stringByAppendingString:d.uniqueID];}
 a.accessoryView=devices;[a addButtonWithTitle:@"Play"];[a addButtonWithTitle:@"Cancel"];
 if([a runModal]!=NSAlertFirstButtonReturn)return;
 NSString *source=devices.selectedItem.representedObject;
 if([source hasPrefix:@"screen:"]){[self openLocation:source];return;}
 AVMediaType type=[source hasPrefix:@"avaudio"]?AVMediaTypeAudio:AVMediaTypeVideo;
 [AVCaptureDevice requestAccessForMediaType:type completionHandler:^(BOOL granted){
  dispatch_async(dispatch_get_main_queue(),^{if(granted)[self openLocation:source];else [self menuNote:@"Capture Device" detail:@"Allow camera or microphone access for RMP in System Settings to use this device."];});
 }];
}
-(void)toggleWallpaper{
 if(wallpaper){
  wallpaper=NO;window.ignoresMouseEvents=NO;window.styleMask=restoreStyle;
  window.collectionBehavior=restoreCollection;window.level=alwaysTop?NSFloatingWindowLevel:NSNormalWindowLevel;
  [window setFrame:restoreFrame display:YES];[window makeKeyAndOrderFront:nil];
 }else{
  if(inFull){[self menuNote:@"Set as Wallpaper" detail:@"Leave fullscreen to use the playback window as your desktop wallpaper."];return;}
  wallpaper=YES;restoreStyle=window.styleMask;restoreCollection=window.collectionBehavior;restoreFrame=window.frame;
  NSScreen *display=window.screen?:NSScreen.mainScreen;
  window.styleMask=NSWindowStyleMaskBorderless;window.collectionBehavior=NSWindowCollectionBehaviorCanJoinAllSpaces|NSWindowCollectionBehaviorStationary|NSWindowCollectionBehaviorIgnoresCycle;
  window.level=CGWindowLevelForKey(kCGDesktopIconWindowLevelKey)-1;
  [window setFrame:display.frame display:YES];window.ignoresMouseEvents=YES;[window orderFront:nil];
 }
 [self arrangeChrome];
}
-(BOOL)applicationShouldHandleReopen:(NSApplication *)app hasVisibleWindows:(BOOL)visible{
 if(wallpaper){[self toggleWallpaper];return NO;}
 [window makeKeyAndOrderFront:nil];return YES;
}
-(void)updateViewControls{
 if(!extraControls)return;
 BOOL regular=!inFull && !minimalInterface && !wallpaper && skin!=LINUX_THEME;
 extraControls.hidden=!regular || !advancedVisible;
 statusLine.hidden=!regular || !statusVisible;
 dockedList.hidden=!regular || !playlistShown || !playlistDocked || [self hasMediaLayout];
 [queuePanel orderOut:nil];
 if(playlistShown && !playlistDocked && regular){[self queueScrollIn:queuePanel.contentView];[queuePanel orderFront:nil];}
 if(!dockedList.hidden){[self queueScrollIn:dockedList];screenRight.constant=-225;}
 if(!regular)screen.hidden=NO;
 bottomInset.constant=-12-(extraControls.hidden?0:30)-(statusLine.hidden?0:18);
 extraBottom.constant=-(statusLine.hidden?0:18);
 if(regular)[queueList reloadData];
}
-(void)queueScrollIn:(NSView *)host{
 if(queueScroll.superview==host)return;[queueScroll removeFromSuperview];[host addSubview:queueScroll];
 queueScroll.frame=host.bounds;queueScroll.autoresizingMask=NSViewWidthSizable|NSViewHeightSizable;
}
-(void)playQueueRow:(id)sender{NSInteger row=queueList.clickedRow;if(row<0)row=queueList.selectedRow;if(row>=0)[self launch:(NSUInteger)row];}
-(void)makeViewControls:(NSView *)root{
 queuePanel=[[NSPanel alloc]initWithContentRect:NSMakeRect(0,0,330,340) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskResizable backing:NSBackingStoreBuffered defer:NO];
 queuePanel.title=@"RMP Playlist";queuePanel.releasedWhenClosed=NO;queuePanel.delegate=self;[queuePanel center];
 queueList=[NSTableView new];NSTableColumn *column=[[NSTableColumn alloc]initWithIdentifier:@"queue"];column.title=@"Current playlist";column.width=278;
 [queueList addTableColumn:column];queueList.delegate=self;queueList.dataSource=self;queueList.target=self;queueList.doubleAction=@selector(playQueueRow:);
 queueScroll=[NSScrollView new];queueScroll.hasVerticalScroller=YES;queueScroll.autohidesScrollers=YES;queueScroll.documentView=queueList;
 dockedList=[NSView new];[root addSubview:dockedList];dockedList.translatesAutoresizingMaskIntoConstraints=NO;
 statusLine=label(@"Ready",11,NO);[root addSubview:statusLine];statusLine.translatesAutoresizingMaskIntoConstraints=NO;
 NSMutableArray *buttons=[NSMutableArray new];
 for(NSArray *pair in @[@[@"Record",@"record"],@[@"Snapshot",@"snapshot"],@[@"−10 s",@"-10"],@[@"+10 s",@"10"]]){
  NSButton *b=[NSButton buttonWithTitle:pair[0] target:self action:@selector(advancedButton:)];b.identifier=pair[1];[buttons addObject:b];}
 extraControls=[self row:buttons];[root addSubview:extraControls];extraControls.translatesAutoresizingMaskIntoConstraints=NO;
 extraBottom=[extraControls.bottomAnchor constraintEqualToAnchor:root.bottomAnchor];
 [NSLayoutConstraint activateConstraints:@[extraBottom,[extraControls.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:12],
  [statusLine.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:12],[statusLine.bottomAnchor constraintEqualToAnchor:root.bottomAnchor constant:-2],
  [dockedList.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],[dockedList.widthAnchor constraintEqualToConstant:222],
  [dockedList.topAnchor constraintEqualToAnchor:screen.topAnchor],[dockedList.bottomAnchor constraintEqualToAnchor:screen.bottomAnchor]]];
 playlistDocked=![NSUserDefaults.standardUserDefaults boolForKey:@"UndockedPlaylist"];
 advancedVisible=[NSUserDefaults.standardUserDefaults boolForKey:@"AdvancedControls"];statusVisible=[NSUserDefaults.standardUserDefaults boolForKey:@"ShowStatusBar"];
}
-(void)advancedButton:(NSButton *)b{
 if([b.identifier hasSuffix:@"10"]){NSString *r=[self engineAction:@"jump" value:b.identifier];if(r.length)[self menuNote:@"Jump" detail:r];return;}
 NSMenuItem *i=[NSMenuItem new];i.title=b.title;i.representedObject=@[b.identifier,@""];[self runMenuCommand:i];
}
-(void)configureView:(BOOL)preferences{
 NSAlert *a=[NSAlert new];a.messageText=preferences?@"RMP Preferences":@"Customize Interface";
 NSButton *(^check)(NSString *,BOOL)=^NSButton *(NSString *title,BOOL yes){NSButton *b=[NSButton checkboxWithTitle:title target:nil action:nil];b.state=yes;return b;};
 NSButton *advanced=check(@"Show advanced playback controls",advancedVisible),*statusBar=check(@"Show status bar",statusVisible),*dock=check(@"Dock playlist in player",playlistDocked);
 NSArray *fields=@[advanced,statusBar,dock];NSSlider *v=nil;
 if(preferences){v=[NSSlider sliderWithValue:volume.doubleValue minValue:0 maxValue:100 target:nil action:nil];fields=[fields arrayByAddingObjectsFromArray:@[label(@"Startup volume",12,NO),v]];}
 NSStackView *form=[NSStackView stackViewWithViews:fields];form.orientation=NSUserInterfaceLayoutOrientationVertical;form.alignment=NSLayoutAttributeLeading;
 form.frame=NSMakeRect(0,0,340,preferences?138:84);a.accessoryView=form;[a addButtonWithTitle:@"Apply"];[a addButtonWithTitle:@"Cancel"];if([a runModal]!=NSAlertFirstButtonReturn)return;
 advancedVisible=advanced.state;statusVisible=statusBar.state;playlistDocked=dock.state;
 NSUserDefaults *prefs=NSUserDefaults.standardUserDefaults;[prefs setBool:advancedVisible forKey:@"AdvancedControls"];[prefs setBool:statusVisible forKey:@"ShowStatusBar"];[prefs setBool:!playlistDocked forKey:@"UndockedPlaylist"];
 if(v){volume.doubleValue=v.doubleValue;[prefs setInteger:volume.integerValue forKey:@"StartupVolume"];[self changeVolume:volume];}
 [self arrangeChrome];
}
-(void)effectsPanel:(BOOL)sync {
 NSAlert *a=[NSAlert new];a.messageText=sync?@"Track Synchronization":@"Effects and Filters";
 NSArray *keys=sync?@[@"audio-delay",@"subtitle-delay"]:@[@"brightness",@"contrast"];
 NSArray *titles=sync?@[@"Audio delay (seconds)",@"Subtitle delay (seconds)"]:@[@"Brightness (0–2)",@"Contrast (0–2)"];
 NSMutableArray *views=[NSMutableArray new],*values=[NSMutableArray new];NSPopUpButton *eq=nil;
 if(!sync){eq=[[NSPopUpButton alloc]initWithFrame:NSMakeRect(0,0,180,26) pullsDown:NO];
  for(NSString *row in [takeTerminalString(ap_menu_options(p,"equalizer")) componentsSeparatedByString:@"\n"]){
   NSArray *pair=[row componentsSeparatedByString:@"\t"];if(pair.count!=2)continue;[eq addItemWithTitle:pair[1]];eq.lastItem.representedObject=pair[0];
  }if(eq.numberOfItems){[views addObject:label(@"Equalizer preset",12,NO)];[views addObject:eq];if(effectValues[@"equalizer"])[eq selectItemAtIndex:[eq indexOfItemWithRepresentedObject:effectValues[@"equalizer"]]];}}
 for(NSUInteger i=0;i<keys.count;i++){
  NSTextField *f=[NSTextField textFieldWithString:effectValues[keys[i]]?: (sync?@"0":@"1")];
  [views addObject:label(titles[i],12,NO)];[views addObject:f];[values addObject:f];
 }
 NSStackView *form=[NSStackView stackViewWithViews:views];form.orientation=NSUserInterfaceLayoutOrientationVertical;form.frame=NSMakeRect(0,0,320,sync?110:174);
 a.accessoryView=form;[a addButtonWithTitle:@"Apply"];[a addButtonWithTitle:@"Cancel"];if([a runModal]!=NSAlertFirstButtonReturn)return;
 if(eq.selectedItem.representedObject){NSString *v=eq.selectedItem.representedObject,*r=[self engineAction:@"equalizer" value:v];if(r.length){[self menuNote:a.messageText detail:r];return;}effectValues[@"equalizer"]=v;}
 for(NSUInteger i=0;i<keys.count;i++){NSString *v=[values[i] stringValue];NSString *r=[self engineAction:keys[i] value:v];if(r.length){[self menuNote:a.messageText detail:r];return;}effectValues[keys[i]]=v;}
}
-(void)runMenuCommand:(NSMenuItem *)sender{
 NSString *key=sender.representedObject[0],*value=sender.representedObject[1];
 if([key isEqual:@"folder"]){[self openFolder];return;}
 if([key isEqual:@"disc"]){NSOpenPanel *picker=[NSOpenPanel openPanel];picker.title=@"Open DVD Disc or DVD Folder";picker.directoryURL=[NSURL fileURLWithPath:@"/Volumes"];picker.canChooseFiles=NO;picker.canChooseDirectories=YES;
  if([picker runModal]==NSModalResponseOK){NSURLComponents *c=[NSURLComponents componentsWithURL:picker.URL resolvingAgainstBaseURL:NO];c.scheme=@"dvdnav";[self accept:@[c.URL]];}return;}
 if([key isEqual:@"capture"]){[self openCapture];return;}
 if([key isEqual:@"clipboard"]){NSPasteboard *pb=NSPasteboard.generalPasteboard;NSURL *file=[[pb readObjectsForClasses:@[NSURL.class] options:@{}]firstObject];if(file){[self accept:@[file]];return;}
  NSString *s=[[pb stringForType:NSPasteboardTypeString]stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet];if(s.length)[self openLocation:s];else[self menuNote:sender.title detail:@"Copy a file path or media URL first."];return;}
 if([key isEqual:@"recent"]){[self openLocation:value];return;}
 if([key isEqual:@"clear-recent"]){[NSUserDefaults.standardUserDefaults removeObjectForKey:@"RecentMedia"];return;}
 if([key isEqual:@"quit-end"]){quitAtEnd=!quitAtEnd;return;}
 if([key isEqual:@"desktop-shortcut"]){BOOL on=!desktopShortcutWanted();setDesktopShortcut(on,NSBundle.mainBundle.bundlePath);sender.state=on?NSControlStateValueOn:NSControlStateValueOff;return;}
 if([key isEqual:@"record"] && recording){NSString *r=[self engineAction:@"record" value:@"stop"];if(r.length)[self menuNote:@"Record" detail:r];else recording=NO;return;}
 if([key isEqual:@"record"]||[key isEqual:@"convert"]||[key isEqual:@"snapshot"]){
  [self pickOutput:key title:sender.title extension:[key isEqual:@"snapshot"]?@"png":[key isEqual:@"record"]?@"ts":@"mp4"];return;}
 if([key isEqual:@"stream-out"]){value=[self askValue:sender.title detail:@"Stream the current media to a UDP receiver." initial:@"udp://127.0.0.1:1234"];if(!value.length)return;key=@"stream";}
 if([key isEqual:@"time"]){value=[self askValue:sender.title detail:@"Enter elapsed seconds or a time such as 1:30." initial:stamp(ap_time(p))];if(!value)return;
  NSArray *parts=[value componentsSeparatedByString:@":"];if(parts.count>1){
   double t=0;for(NSString *component in parts){NSScanner *scanner=[NSScanner scannerWithString:component];double v;if(![scanner scanDouble:&v]||!scanner.isAtEnd||v<0){[self menuNote:sender.title detail:@"Enter a valid playback time."];return;}t=t*60+v;}value=[@(t)stringValue];}}
 if([key isEqual:@"volume-step"]){volume.doubleValue=MIN(100,MAX(0,volume.doubleValue+value.intValue));[self changeVolume:volume];return;}
 if([key isEqual:@"wallpaper"]){[self toggleWallpaper];return;}
 if([key isEqual:@"fit"]){key=@"zoom";value=@"0";}
 if([key isEqual:@"subtitle-file"]){NSOpenPanel *pick=[NSOpenPanel openPanel];pick.title=@"Add Subtitle File";pick.allowedFileTypes=@[@"srt",@"vtt",@"ass",@"ssa",@"sub",@"idx",@"ttml",@"smi"];if([pick runModal]!=NSModalResponseOK)return;value=pick.URL.path;}
 if([key isEqual:@"effects"]||[key isEqual:@"sync"]){[self effectsPanel:[key isEqual:@"sync"]];return;}
 if([key isEqual:@"customize"]||[key isEqual:@"preferences"]){[self configureView:[key isEqual:@"preferences"]];return;}
 if([key isEqual:@"terminal"]){[self pickTheme:[themeMenu itemWithTag:LINUX_THEME]];return;}
 if([key isEqual:@"top"]){alwaysTop=!alwaysTop;if(!wallpaper)window.level=alwaysTop?NSFloatingWindowLevel:NSNormalWindowLevel;return;}
 if([key isEqual:@"minimal"]){minimalInterface=!minimalInterface;[self arrangeChrome];return;}
 if([key isEqual:@"advanced"]){advancedVisible=!advancedVisible;[NSUserDefaults.standardUserDefaults setBool:advancedVisible forKey:@"AdvancedControls"];[self arrangeChrome];return;}
 if([key isEqual:@"status"]){statusVisible=!statusVisible;[NSUserDefaults.standardUserDefaults setBool:statusVisible forKey:@"ShowStatusBar"];[self arrangeChrome];return;}
 if([key isEqual:@"docked"]){playlistDocked=!playlistDocked;[NSUserDefaults.standardUserDefaults setBool:!playlistDocked forKey:@"UndockedPlaylist"];[self arrangeChrome];return;}
 if([key isEqual:@"playlist"]){
  if(playlistDocked&&[self hasMediaLayout]){mediaPage=mediaPage==2?0:2;[self arrangeChrome];return;}
  playlistShown=!playlistShown;[self arrangeChrome];return;
 }
 if([key isEqual:@"vlsub"]){
  NSAlert *a=[NSAlert new];a.messageText=@"Search for Subtitles";a.informativeText=@"Open a browser search on OpenSubtitles using the current media title. After downloading a subtitle, choose Subtitle → Add Subtitle File.";
  [a addButtonWithTitle:@"Search in Browser"];[a addButtonWithTitle:@"Cancel"];if([a runModal]!=NSAlertFirstButtonReturn)return;
  NSString *result=[self engineAction:@"subtitle-search" value:@""];NSURL *link=[NSURL URLWithString:result];if([link.scheme isEqual:@"https"]){[NSWorkspace.sharedWorkspace openURL:link];}else [self menuNote:@"Subtitles" detail:result];return;
 }
 if([key isEqual:@"help"]){[NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:@"https://github.com/KMThegamer2022/RMP#readme"]];return;}
 NSString *result=[self engineAction:key value:value];
 if([key isEqual:@"output-stop"]&&!result.length)recording=NO;
 if(result.length)[self menuNote:sender.title detail:result];
}
-(BOOL)validateMenuItem:(NSMenuItem *)entry{
 if(entry.action!=@selector(runMenuCommand:))return YES;
 NSString *key=entry.representedObject[0];
 NSDictionary *flags=@{@"quit-end":@(quitAtEnd),@"top":@(alwaysTop),@"wallpaper":@(wallpaper),@"minimal":@(minimalInterface),
  @"advanced":@(advancedVisible),@"status":@(statusVisible),@"playlist":@(playlistShown||([self hasMediaLayout]&&mediaPage==2)),@"docked":@(playlistDocked),@"record":@(recording)};
 if(flags[key])entry.state=[flags[key] boolValue]?NSControlStateValueOn:NSControlStateValueOff;
 if([key isEqual:@"snapshot"])return p&&ap_has_video(p);
 if([@[@"jump",@"time"] containsObject:key])return p&&ap_seekable(p);
 if([@[@"record",@"convert",@"stream-out",@"subtitle-file",@"vlsub"] containsObject:key])return p&&ap_index(p)>=0;
 return YES;
}
