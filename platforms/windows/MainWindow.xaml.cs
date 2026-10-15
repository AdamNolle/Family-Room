using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Dispatching;
using System.Text.Json.Nodes;
using System.Diagnostics;
using System.Globalization;
using Windows.Media.Core;
using Windows.Media.Playback;
using Windows.ApplicationModel.DataTransfer;
using Windows.Storage.Pickers;
using Windows.Networking.Connectivity;
namespace FamilyRoom;
public partial class MainWindow : Window {
    private NativeCore? core; private JsonNode? snapshot; private string section="home"; private string momentRiverRoomId=""; private string momentRiverYear="";
    private const int MaxBulkSelection=500;
    private bool selectingAssets;
    private readonly HashSet<string> selectedAssets=new(StringComparer.Ordinal);
    private readonly HashSet<string> selectedLibraryPeople=new(StringComparer.Ordinal);
    private bool matchAllLibraryPeople;
    private readonly List<JsonNode> screeningQueue=new();
    private MediaPlayer? screeningMediaPlayer;
    private readonly List<ScreeningCaptionCue> screeningCaptions=new();
    private DispatcherQueueTimer? screeningCaptionTimer;
    private DispatcherQueueTimer? folderWatchTimer;
    private bool scanningFolderWatches;
    private string screeningTitle="";
    private string screeningRoomId="";
    private int screeningIndex;
    private int screeningGeneration;
    private ulong screeningStartMs;
    private string ownershipOfferRoomId="";
    private string ownershipOfferDevice=""; private string? reviewedInvitation;
    private bool refreshingLibraryFilters;
 private JsonArray? momentSuggestions;
 private string momentSuggestionsRoomId="";
 private JsonNode? photoDuplicateResponse;
 private string photoDuplicateRoomId="";
 private bool scanningPhotoDuplicates;
    private string? lastVideoPrompt;

    private sealed record ScreeningCaptionCue(long StartMs,long EndMs,string Text)
    {
        public bool IsActiveAt(long milliseconds)=>StartMs<=milliseconds&&milliseconds<EndMs;
    }
    public MainWindow() {
        InitializeComponent();
        Title="Family Room";
        Closed+=(_,_)=>{RecoveryKeyText.Text="";RecoveryKeyInput.Password="";screeningGeneration++;screeningCaptionTimer?.Stop();folderWatchTimer?.Stop();screeningMediaPlayer?.Dispose();core?.Dispose();};
        _=Open();
        folderWatchTimer=DispatcherQueue.GetForCurrentThread().CreateTimer();
        folderWatchTimer.Interval=TimeSpan.FromSeconds(15);
        folderWatchTimer.Tick+=async(_,_)=>await ScanFolderWatches();
        folderWatchTimer.Start();
    }
    private async Task Open() { try { core=await Task.Run(()=>new NativeCore()); await Reload(); } catch(Exception e) { Status.Text=e.Message; } }
    private JsonNode? Room => snapshot?["rooms"]?.AsArray().FirstOrDefault(r=>r?["id"]?.GetValue<string>()==Rooms.SelectedValue?.ToString());
    private async Task Execute(object request) { try { if(core==null) return; await core.Command(request); await Reload(); Status.Text="Saved privately."; } catch(Exception e) { Status.Text=e.Message; } }
    private static string NetworkState() {
        var profile=NetworkInformation.GetInternetConnectionProfile();
        if(profile==null || profile.GetNetworkConnectivityLevel()==NetworkConnectivityLevel.None) return "offline";
        var cost=profile.GetConnectionCost();
        if(cost.Roaming || cost.OverDataLimit || cost.BackgroundDataUsageRestricted) return "metered";
        return cost.NetworkCostType switch {
            NetworkCostType.Unrestricted => "unmetered",
            NetworkCostType.Fixed or NetworkCostType.Variable => "metered",
            _ => "unknown"
        };
    }
    private async Task UpdateNetworkState() {
        if(core!=null) await core.Command(new {action="set_network_state",state=NetworkState()});
    }
    private async Task Reload() {
        if(core==null) return; var selected=Rooms.SelectedValue?.ToString(); snapshot=await core.Command(new {action="snapshot"});
        Rooms.ItemsSource=snapshot["rooms"]!.AsArray().Select(r=>new RoomChoice(r!["id"]!.GetValue<string>(),r["name"]!.GetValue<string>())).ToList(); Rooms.DisplayMemberPath="Name"; Rooms.SelectedValuePath="Id"; Rooms.SelectedValue=selected; if(Rooms.SelectedIndex<0) Rooms.SelectedIndex=0;
        DeviceCode.Text=snapshot["device_id"]!.GetValue<string>(); RefreshRecoveryAvailability(); Render();
    }
    private bool CanRestoreBackup() =>
        snapshot is JsonObject catalog && catalog["rooms"] is JsonArray rooms && rooms.Count==0 &&
        catalog["pending_invitations"] is JsonArray invitations && invitations.Count==0;
    private void RefreshRecoveryAvailability() =>
        RestoreBackupButton.IsEnabled=CanRestoreBackup() && !string.IsNullOrWhiteSpace(RecoveryKeyInput.Password);
    private void RecoveryKeyChanged(object sender,RoutedEventArgs args) => RefreshRecoveryAvailability();
    private void RevealRecoveryKey(object sender,RoutedEventArgs args) {
        if(RecoveryKeyText.Visibility==Visibility.Visible) {
            RecoveryKeyText.Text=""; RecoveryKeyText.Visibility=Visibility.Collapsed; RevealRecoveryKeyButton.Content="Show recovery key"; return;
        }
        try {
            if(core==null) throw new InvalidOperationException("The encrypted library is still opening.");
            RecoveryKeyText.Text=core.RevealRecoveryKey(); RecoveryKeyText.Visibility=Visibility.Visible; RevealRecoveryKeyButton.Content="Hide recovery key";
        } catch(Exception error) { Status.Text=$"Could not read the recovery key: {error.Message}"; }
    }
    private async void SaveEncryptedBackup(object sender,RoutedEventArgs args) {
        if(core==null) return;
        try {
            var picker=new FileSavePicker();
            WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this));
            picker.SuggestedFileName="Family-Room.frbackup";
            picker.FileTypeChoices.Add("Encrypted Family Room backup",new List<string>{".frbackup"});
            var file=await picker.PickSaveFileAsync();
            if(file==null) return;
            await core.Command(new {action="backup",path=file.Path});
            Status.Text="Encrypted backup saved.";
        } catch(Exception error) { Status.Text=$"Backup failed: {error.Message}"; }
    }
    private async void RestoreEncryptedBackup(object sender,RoutedEventArgs args) {
        if(core==null) return;
        if(!CanRestoreBackup()) { Status.Text="Restore is available only when this library has no Rooms or pending invitations."; return; }
        var recoveryKey=RecoveryKeyInput.Password.Trim();
        if(recoveryKey.Length==0) { Status.Text="Enter the recovery key for this backup."; return; }
        try {
            var picker=new FileOpenPicker();
            WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this));
            picker.FileTypeFilter.Add(".frbackup");
            var file=await picker.PickSingleFileAsync();
            if(file==null) return;
            await core.Command(new {action="restore",path=file.Path,recovery_key=recoveryKey});
            RecoveryKeyInput.Password="";
            await Reload();
            Status.Text="Backup restored. Review storage sources before syncing.";
        } catch(Exception error) { Status.Text=$"Restore failed: {error.Message}"; }
    }
    private static bool MatchesLibrarySearch(JsonNode item, JsonNode room, string query) {
        var terms=query.Split((char[]?)null,StringSplitOptions.RemoveEmptyEntries);
        if(terms.Length==0) return true;
        var peopleIds=new HashSet<string>(StringComparer.Ordinal);
        foreach(var personId in item["people"]?.AsArray()??new JsonArray()) {
            if(personId is JsonValue value && value.TryGetValue<string>(out var id) && !string.IsNullOrEmpty(id)) peopleIds.Add(id);
        }
        var peopleNames=(room["people"]?.AsArray()??new JsonArray())
            .Where(person=>person!=null && peopleIds.Contains(person?["id"]?.GetValue<string>()??""))
            .Select(person=>person?["name"]?.GetValue<string>()??"");
        var searchable=string.Join(" ",
            item["caption"]?.GetValue<string>()??"",
            item["filename"]?.GetValue<string>()??"",
            item["kind"]?.GetValue<string>()??"",
            item["title"]?.GetValue<string>()??"",
            item["name"]?.GetValue<string>()??"",
            string.Join(" ",peopleNames));
        return terms.All(term=>searchable.Contains(term,StringComparison.OrdinalIgnoreCase));
    }
    private static string? CaptureYear(JsonNode? asset) {
        if(!long.TryParse(asset?["captured_at"]?.ToString(),out var timestamp)) return null;
        try { return DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().Year.ToString(); }
        catch(ArgumentOutOfRangeException) { return null; }
    }
    private void RefreshLibraryFilters(JsonNode room) {
        var selectedYear=YearFilter.SelectedValue?.ToString()??"";
        var selectedMoment=MomentFilter.SelectedValue?.ToString()??"";
        refreshingLibraryFilters=true;
        try {
            YearFilter.Items.Clear();
            YearFilter.Items.Add(new ComboBoxItem {Content="All years",Tag=""});
            var years=(room["assets"]?.AsArray()??new JsonArray())
                .Where(asset=>asset?["deleted"]?.GetValue<bool>()!=true)
                .Select(CaptureYear).Where(year=>year!=null).Distinct().OrderByDescending(year=>year).ToList();
            foreach(var year in years) YearFilter.Items.Add(new ComboBoxItem {Content=year,Tag=year});
            YearFilter.SelectedValue=years.Contains(selectedYear)?selectedYear:"";

            MomentFilter.Items.Clear();
            MomentFilter.Items.Add(new ComboBoxItem {Content="All Moments",Tag=""});
            var moments=(room["moments"]?.AsArray()??new JsonArray())
                .Where(moment=>moment?["id"]!=null)
                .Select(moment=>(Id:moment!["id"]!.GetValue<string>(),Title:moment["title"]?.GetValue<string>()??"Moment",Start:moment["start"]?.GetValue<long>()??0))
                .OrderByDescending(moment=>moment.Start).ToList();
            foreach(var moment in moments) MomentFilter.Items.Add(new ComboBoxItem {Content=moment.Title,Tag=moment.Id});
            MomentFilter.SelectedValue=moments.Any(moment=>moment.Id==selectedMoment)?selectedMoment:"";

            var people=(room["people"]?.AsArray()??new JsonArray())
                .Where(person=>person?["id"]!=null)
                .Select(person=>(Id:person!["id"]!.GetValue<string>(),Name:person["name"]?.GetValue<string>()??"Person"))
                .OrderBy(person=>person.Name,StringComparer.OrdinalIgnoreCase).ToList();
            var availablePeople=people.Select(person=>person.Id).ToHashSet(StringComparer.Ordinal);
            selectedLibraryPeople.RemoveWhere(id=>!availablePeople.Contains(id));
            PersonFilters.Children.Clear();
            foreach(var person in people) {
                var id=person.Id;
                var filter=new CheckBox {Content=person.Name,IsChecked=selectedLibraryPeople.Contains(id),VerticalAlignment=VerticalAlignment.Center};
                filter.Checked+=(_,_)=>SetLibraryPersonFilter(id,true);
                filter.Unchecked+=(_,_)=>SetLibraryPersonFilter(id,false);
                PersonFilters.Children.Add(filter);
            }
            MatchAllPeople.IsChecked=matchAllLibraryPeople;
            MatchAllPeople.IsEnabled=selectedLibraryPeople.Count>0;
        } finally {
            refreshingLibraryFilters=false;
        }
    }
    private bool MatchesLibraryFilters(JsonNode asset,JsonNode room) {
        var selectedYear=YearFilter.SelectedValue?.ToString();
        if(!string.IsNullOrEmpty(selectedYear) && CaptureYear(asset)!=selectedYear) return false;
        var selectedMoment=MomentFilter.SelectedValue?.ToString();
        if(!string.IsNullOrEmpty(selectedMoment) && !(room["moments"]?.AsArray()??new JsonArray()).Any(moment=>
            moment!=null && moment["id"]?.GetValue<string>()==selectedMoment &&
            (moment["asset_ids"]?.AsArray()??new JsonArray()).Any(id=>id?.GetValue<string>()==asset["id"]?.GetValue<string>()))) return false;
        if(selectedLibraryPeople.Count==0) return true;
        var taggedPeople=new HashSet<string>(StringComparer.Ordinal);
        foreach(var personId in asset["people"]?.AsArray()??new JsonArray()) {
            if(personId is JsonValue value && value.TryGetValue<string>(out var id)) taggedPeople.Add(id);
        }
        return matchAllLibraryPeople
            ? selectedLibraryPeople.All(taggedPeople.Contains)
            : selectedLibraryPeople.Any(taggedPeople.Contains);
    }
    private void LibraryFilterChanged(object sender,SelectionChangedEventArgs args) {
        if(!refreshingLibraryFilters) Render();
    }
    private void SetLibraryPersonFilter(string personId,bool selected) {
        if(refreshingLibraryFilters) return;
        if(selected) selectedLibraryPeople.Add(personId);
        else selectedLibraryPeople.Remove(personId);
        if(selectedLibraryPeople.Count==0) matchAllLibraryPeople=false;
        Render();
    }
    private void PeopleMatchChanged(object sender,RoutedEventArgs args) {
        if(refreshingLibraryFilters) return;
        matchAllLibraryPeople=MatchAllPeople.IsChecked==true;
        Render();
    }
    private void ClearPeopleFilter(object sender,RoutedEventArgs args) {
        selectedLibraryPeople.Clear();
        matchAllLibraryPeople=false;
        Render();
    }
    private static string SetupMark(bool complete)=>complete ? "✓" : "○";
    private void RenderRoomStorageAccess() {
        var room=Room;
        var role=room?["role"]?.GetValue<string>()??"";
        var canManage=role=="owner" || role=="admin";
        RoomStorageControls.Visibility=canManage ? Visibility.Visible : Visibility.Collapsed;
        RoomStorageOwnerNote.Visibility=room!=null && !canManage ? Visibility.Visible : Visibility.Collapsed;
        RoomStorageOwnerNote.Text="The Room owner manages shared storage and upload destinations. You can join without an account on the owner's storage provider.";
    }
    private void RenderRoomSetup() {
        if(section!="home" && section!="settings") { RoomSetupPanel.Visibility=Visibility.Collapsed; return; }
        var room=Room;
        if(room==null) { RoomSetupPanel.Visibility=Visibility.Collapsed; return; }
        RoomSetupPanel.Visibility=Visibility.Visible;
        var role=room["role"]?.GetValue<string>()??"viewer";
        if(role!="owner" && role!="admin") {
            RoomSetupSummary.Text="The Room owner manages shared storage and upload destinations. You can join without an account on the owner's storage provider.";
            RoomSetupStorage.Visibility=Visibility.Collapsed;
            RoomSetupUploads.Visibility=Visibility.Collapsed;
            RoomSetupRecovery.Visibility=Visibility.Collapsed;
            RoomSetupContribution.Visibility=Visibility.Collapsed;
            return;
        }

        var roomId=room["id"]?.GetValue<string>()??"";
        var sources=snapshot?["sources"]?.AsArray()??new JsonArray();
        var hasPreferredStorage=sources.Any(source=>
            source!=null && source["room_id"]?.GetValue<string>()==roomId &&
            source["preferred"]?.GetValue<bool>()==true && source["paused"]?.GetValue<bool>()!=true);
        var hasContribution=(room["assets"]?.AsArray()??new JsonArray()).Any(asset=>
            asset!=null && asset["deleted"]?.GetValue<bool>()!=true &&
            (asset["kind"]?.GetValue<string>()=="photo" || asset["kind"]?.GetValue<string>()=="video"));
        var hasActiveFolderWatch=(snapshot?["watches"]?.AsArray()??new JsonArray()).Any(watch=>
            watch!=null && watch["room_id"]?.GetValue<string>()==roomId &&
            watch["paused"]?.GetValue<bool>()!=true &&
            watch["path"]?.GetValue<string>() is string path && !path.StartsWith("photos://",StringComparison.Ordinal));
        var recoveryKeyAvailable=core!=null;
        var completeCount=new[]{hasPreferredStorage,recoveryKeyAvailable,hasContribution,hasActiveFolderWatch}.Count(value=>value);

        RoomSetupSummary.Text=$"{completeCount} of 4 setup checks are ready. Choose where Room originals live, keep recovery details safe, and add your first memories.";
        RoomSetupStorage.Text=$"{SetupMark(hasPreferredStorage)} Choose preferred storage · {(hasPreferredStorage ? "A destination is ready for this Room." : "Continue to Upload destinations below to connect a source.")}";
        RoomSetupUploads.Text=$"{SetupMark(hasActiveFolderWatch)} Automatic folder uploads · {(hasActiveFolderWatch ? "This Room's folder is checked while Family Room is open." : "Choose a folder below. Windows checks it while Family Room is open; camera-roll uploads are unavailable.")}";
        RoomSetupRecovery.Text=$"{SetupMark(recoveryKeyAvailable)} Recovery key available on this device · Reveal it from Recovery & backup and store it separately before device loss.";
        RoomSetupContribution.Text=$"{SetupMark(hasContribution)} Add first photos or videos · {(hasContribution ? "This Room already has a contribution." : "Use Import media above to add the first memories.")}";
        RoomSetupStorage.Visibility=Visibility.Visible;
        RoomSetupUploads.Visibility=Visibility.Visible;
        RoomSetupRecovery.Visibility=Visibility.Visible;
        RoomSetupContribution.Visibility=Visibility.Visible;
    }
    private void RenderFolderWatches() {
        var visible=section=="home" || section=="settings";
        var room=Room;
        FolderUploadPanel.Visibility=visible && room!=null ? Visibility.Visible : Visibility.Collapsed;
        if(!visible || room==null) return;
        var roomId=room["id"]?.GetValue<string>()??"";
        var roomName=room["name"]?.GetValue<string>()??"this Room";
        var role=room["role"]?.GetValue<string>()??"viewer";
        var canContribute=role!="viewer" && role!="removed";
        FolderUploadDescription.Text=$"Choose a folder whose new photos and videos directly inside it should be added to {roomName}. Windows checks watched folders while Family Room is open; it does not run as a background service after you quit.";
        FolderUploadControls.Visibility=canContribute ? Visibility.Visible : Visibility.Collapsed;
        FolderUploadMemberNote.Visibility=canContribute ? Visibility.Collapsed : Visibility.Visible;
        FolderUploadMemberNote.Text="Your Room role does not allow contributions. Ask an owner to update your access.";

        var controls=new List<UIElement>();
        foreach(var watch in snapshot?["watches"]?.AsArray()??new JsonArray()) {
            if(watch==null || watch["room_id"]?.GetValue<string>()!=roomId) continue;
            var path=watch["path"]?.GetValue<string>()??"";
            if(path.StartsWith("photos://",StringComparison.Ordinal)) continue;
            var id=watch["id"]?.GetValue<string>()??"";
            var paused=watch["paused"]?.GetValue<bool>()==true;
            var card=new StackPanel {Spacing=6,Margin=new Thickness(0,8,0,8)};
            card.Children.Add(new TextBlock {Text=path,TextWrapping=TextWrapping.Wrap,FontSize=15});
            var row=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            var extensions=(watch["excluded_extensions"]?.AsArray()??new JsonArray())
                .Where(value=>value!=null).Select(value=>value!.GetValue<string>());
            var exclusionTextBox=new TextBox {Text=string.Join(", ",extensions),PlaceholderText="Excluded extensions"};
            exclusionTextBox.HorizontalAlignment=HorizontalAlignment.Stretch;
            row.Children.Add(exclusionTextBox);
            var save=new Button {Content="Save exclusions"};
            save.Click+=async(_,_)=>await SaveFolderWatchExclusions(id,exclusionTextBox.Text);
            row.Children.Add(save);
            var toggle=new Button {Content=paused ? "Resume" : "Pause"};
            toggle.Click+=async(_,_)=>await SetFolderWatchPaused(id,!paused);
            row.Children.Add(toggle);
            card.Children.Add(row);
            controls.Add(card);
        }
        FolderWatches.ItemsSource=controls;
    }
    private async Task SaveFolderWatchExclusions(string id,string input) {
        if(core==null) return;
        try {
            var extensions=input.Split(',',StringSplitOptions.RemoveEmptyEntries|StringSplitOptions.TrimEntries);
            await core.Command(new {action="set_watch_exclusions",id,excluded_extensions=extensions});
            await Reload();
            FolderWatchStatus.Text="Upload exclusions saved.";
        } catch(Exception error) { FolderWatchStatus.Text=$"Could not save exclusions: {error.Message}"; }
    }
    private async Task SetFolderWatchPaused(string id,bool paused) {
        if(core==null) return;
        try {
            await core.Command(new {action="pause_watch",id,paused});
            await Reload();
            FolderWatchStatus.Text=paused ? "Automatic uploads paused for this folder." : "Automatic uploads resumed.";
        } catch(Exception error) { FolderWatchStatus.Text=$"Could not update folder uploads: {error.Message}"; }
    }
    private async void AddFolderWatch(object sender,RoutedEventArgs args) {
        var room=Room;
        if(core==null || room==null || room["role"]?.GetValue<string>()=="viewer") return;
        try {
            var picker=new FolderPicker {SuggestedStartLocation=PickerLocationId.PicturesLibrary};
            picker.FileTypeFilter.Add("*");
            WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this));
            var folder=await picker.PickSingleFolderAsync();
            if(folder==null) return;
            var roomId=room["id"]!.GetValue<string>();
            var extensions=(ExcludedWatchExtensions.Text??"").Split(',',StringSplitOptions.RemoveEmptyEntries|StringSplitOptions.TrimEntries);
            await core.Command(new {
                action="watch_folder",room_id=roomId,path=folder.Path,
                include_existing=IncludeExistingWatch.IsChecked==true,
                excluded_extensions=extensions
            });
            await Reload();
            FolderWatchStatus.Text="Folder connected. Checking it for new files…";
            await ScanFolderWatches();
        } catch(Exception error) { FolderWatchStatus.Text=$"Could not start automatic uploads: {error.Message}"; }
    }
    private async Task ScanFolderWatches() {
        if(core==null || scanningFolderWatches) return;
        var watches=snapshot?["watches"]?.AsArray()??new JsonArray();
        var active=watches.Where(watch=>watch!=null && watch["paused"]?.GetValue<bool>()!=true)
            .Where(watch=>!(watch["path"]?.GetValue<string>()??"").StartsWith("photos://",StringComparison.Ordinal)).ToList();
        if(active.Count==0) return;
        scanningFolderWatches=true;
        try {
            var scan=await core.Command(new {action="scan_watches"});
            var imported=scan["imported"]?.GetValue<int>()??0;
            var errors=new List<string>();
            if(scan["errors"] is JsonArray scanErrors) errors.AddRange(scanErrors.Where(error=>error!=null).Select(error=>error!.GetValue<string>()));
            if(imported>0) {
                await UpdateNetworkState();
                foreach(var roomId in active.Select(watch=>watch!["room_id"]?.GetValue<string>()??"").Where(id=>id.Length>0).Distinct()) {
                    var result=await core.Command(new {action="sync_all",room_id=roomId});
                    if(result["errors"] is JsonArray syncErrors) errors.AddRange(syncErrors.Where(error=>error!=null).Select(error=>error!.GetValue<string>()));
                }
                await Reload();
                FolderWatchStatus.Text=$"Automatic uploads found {imported} new item(s). {(errors.Count==0 ? "Sync queued." : string.Join(" · ",errors))}";
            } else if(errors.Count>0) FolderWatchStatus.Text=$"Automatic folder upload needs attention: {string.Join(" · ",errors)}";
        } catch(Exception error) { FolderWatchStatus.Text=$"Automatic folder scan failed: {error.Message}"; }
        finally { scanningFolderWatches=false; }
    }
    private void Render() {
        VideoFinderPanel.Visibility=section=="video-finder"?Visibility.Visible:Visibility.Collapsed;
        RecoveryPanel.Visibility=section=="settings"?Visibility.Visible:Visibility.Collapsed;
        RenderRoomSetup();
        RenderFolderWatches();
        RenderRoomStorageAccess();
        if(section=="settings") { LibraryFilters.Visibility=Visibility.Collapsed; ContentItems.ItemsSource=Array.Empty<UIElement>(); RefreshRecoveryAvailability(); return; }
        if(Room?["role"]?.GetValue<string>() is "owner" or "admin") RenderStorageSources();
        else StorageSources.ItemsSource=Array.Empty<UIElement>();
        RenderMembers();
        if(ContentItems==null) return; var controls=new List<UIElement>(); var room=Room; if(room==null) { ContentItems.ItemsSource=controls; return; }
        LibraryFilters.Visibility=section=="library"?Visibility.Visible:Visibility.Collapsed;
RefreshLibraryFilters(room);
if(section=="home") RenderOnThisDay(controls,room);
if(section=="home" && CanEditAssets(room)) RenderMomentSuggestions(controls,room);
 if(section=="home") RenderMomentRiver(controls,room);
 if(section=="library") RenderPhotoDuplicateReview(controls,room);
        if(section=="albums" || section=="theater") {
            RenderCollections(controls, room, section=="theater");
            if(section=="albums") { ContentItems.ItemsSource=controls; return; }
        }
        var field=section switch {"albums"=>"albums","people"=>"people","theater"=>"stories",_=>"assets"}; if(section=="people") RenderPersonMerge(controls,room);
        if(section=="library" || section=="trash") RenderLibrarySelection(controls,room);
        if(section=="trash" && !(room["assets"]?.AsArray()??new JsonArray()).Any(asset=>asset?["deleted"]?.GetValue<bool>()==true))
            controls.Add(new TextBlock {Text="Recently Deleted is empty. Original files remain safe in the Room.",Opacity=0.72,TextWrapping=TextWrapping.Wrap,Margin=new Thickness(0,8,0,12)});
        foreach(var item in room[field]!.AsArray()) {
            if(item==null) continue;
            if(field=="assets" && (item["deleted"]?.GetValue<bool>()==true)!=(section=="trash")) continue;
            if(field=="assets" && section=="library" && !MatchesLibraryFilters(item,room)) continue;
            var title=(item["caption"]?.GetValue<string>() is string caption && caption.Length>0) ? caption : item["filename"]?.GetValue<string>()??item["title"]?.GetValue<string>()??item["name"]?.GetValue<string>()??"Memory";
            if(!MatchesLibrarySearch(item,room,Search.Text??"")) continue;
            var panel=new StackPanel {Spacing=6,Margin=new Thickness(0,0,0,14)}; panel.Children.Add(new TextBlock {Text=title,FontSize=18});
            if(field=="assets") {
                if((section=="library" || section=="trash") && selectingAssets) {
                    var assetIdForSelection=item["id"]?.GetValue<string>()??"";
                    var selected=new CheckBox {Content="Select",IsChecked=selectedAssets.Contains(assetIdForSelection),
                        IsEnabled=CanEditAssets(room) && (selectedAssets.Contains(assetIdForSelection) || selectedAssets.Count<MaxBulkSelection)};
                    selected.Checked+=(_,_)=>SetAssetSelected(assetIdForSelection,true);
                    selected.Unchecked+=(_,_)=>SetAssetSelected(assetIdForSelection,false);
                    panel.Children.Add(selected);
                }
                panel.Children.Add(new TextBlock {Text=item["availability"]?.GetValue<string>()??""});
                var button=new Button {Content="Open original"}; var roomId=room["id"]!.GetValue<string>();
                button.Click+=async(_,_)=>await OpenOriginal(item,roomId);
                panel.Children.Add(button);
                if(section=="library" && item["kind"]?.GetValue<string>()=="video" && item["deleted"]?.GetValue<bool>()!=true) {
                    var captions=new Button {Content="Export spoken captions"};
                    captions.Click+=async(_,_)=>await ExportVideoTranscript(item,roomId);
                    panel.Children.Add(captions);
                }
                var assetId=item["id"]!.GetValue<string>();
                var likedReactions=room["reactions"]?.AsArray()
                    .Where(reaction=>reaction?["asset_id"]?.GetValue<string>()==assetId && reaction["liked"]?.GetValue<bool>()==true).ToList() ?? new List<JsonNode?>();
                var liked=likedReactions.Any(reaction=>reaction?["author"]?.GetValue<string>()==snapshot?["device_id"]?.GetValue<string>());
                var like=new Button {Content=$"{(liked ? "Unlike" : "Like")} · {likedReactions.Count}",IsEnabled=room["role"]?.GetValue<string>()!="removed"};
                like.Click+=async(_,_)=>await Execute(new {action="set_like",room_id=roomId,asset_id=assetId,liked=!liked});
                panel.Children.Add(like);
                var assetNotes=(room["comments"]?.AsArray()??new JsonArray()).Where(note=>note?["asset_id"]?.GetValue<string>()==assetId).ToList();
                if(assetNotes.Count>0) panel.Children.Add(new TextBlock {Text="Family notes",FontWeight=Microsoft.UI.Text.FontWeights.SemiBold,Margin=new Thickness(0,6,0,0)});
                foreach(var note in assetNotes) {
                    var author=note?["author"]?.GetValue<string>()??"Member";
                    var currentDevice=snapshot?["device_id"]?.GetValue<string>();
                    var attribution=author==currentDevice ? "You" : $"Member {(author.Length>8?author[..8]:author)}";
                    var noteContent=new StackPanel {Spacing=3};
                    noteContent.Children.Add(new TextBlock {Text=note?["body"]?.GetValue<string>()??"",TextWrapping=TextWrapping.Wrap});
                    noteContent.Children.Add(new TextBlock {Text=attribution,FontSize=12,Opacity=0.7});
                    panel.Children.Add(new Border {Padding=new Thickness(12),CornerRadius=new CornerRadius(12),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(38,120,100,80)),Child=noteContent});
                }
                var noteInput=new TextBox {PlaceholderText="Leave a family note",AcceptsReturn=true,TextWrapping=TextWrapping.Wrap,MinHeight=54,MaxLength=10000};
                var postNote=new Button {Content="Post note",IsEnabled=false};
                noteInput.TextChanged+=(_,_)=>postNote.IsEnabled=room["role"]?.GetValue<string>()!="removed"&&!string.IsNullOrWhiteSpace(noteInput.Text)&&System.Text.Encoding.UTF8.GetByteCount(noteInput.Text)<=10000;
                postNote.Click+=async(_,_)=>{
                    if(core==null) return;
                    try {
                        await core.Command(new {action="comment",room_id=roomId,asset_id=assetId,body=noteInput.Text});
                        noteInput.Text="";
                        await Reload();
                    } catch(Exception error) { Status.Text=$"Could not post note: {error.Message}"; }
                };
                panel.Children.Add(noteInput);
                panel.Children.Add(postNote);
                var historyPanel=new StackPanel {Spacing=8,Visibility=Visibility.Collapsed};
                var historyButton=new Button {Content="Caption history"};
                historyButton.Click+=async(_,_)=>{
                    if(historyPanel.Visibility==Visibility.Visible) {
                        historyPanel.Visibility=Visibility.Collapsed;
                        historyButton.Content="Caption history";
                        return;
                    }
                    if(core==null) return;
                    historyButton.IsEnabled=false;
                    try {
                        var response=await core.Command(new {action="asset_history",room_id=roomId,asset_id=assetId});
                        var revisions=response["history"]?.AsArray()??new JsonArray();
                        historyPanel.Children.Clear();
                        for(var index=0;index<revisions.Count;index++) {
                            var revision=revisions[index];
                            var caption=revision?["caption"]?.GetValue<string>()??"";
                            var author=revision?["author"]?.GetValue<string>()??"Member";
                            var attribution=author==snapshot?["device_id"]?.GetValue<string>() ? "You" : $"Member {(author.Length>8?author[..8]:author)}";
                            var revisionCard=new StackPanel {Spacing=4};
                            revisionCard.Children.Add(new TextBlock {Text=string.IsNullOrEmpty(caption)?"(No caption)":caption,TextWrapping=TextWrapping.Wrap});
                            revisionCard.Children.Add(new TextBlock {Text=$"{attribution} · revision {revision?["clock"]?.ToString()??"0"}",FontSize=12,Opacity=0.7});
                            var canEditCaption=room["role"]?.GetValue<string>() is "owner" or "admin" or "contributor";
                            if(caption!=item["caption"]?.GetValue<string>() && canEditCaption) {
                                var useCaption=new Button {Content="Use this caption",HorizontalAlignment=HorizontalAlignment.Left};
                                useCaption.Click+=async(_,_)=>{
                                    try {
                                        await core.Command(new {action="patch_asset",room_id=roomId,asset_id=assetId,fields=new {caption}});
                                        await Reload();
                                    } catch(Exception error) { Status.Text=$"Could not restore caption: {error.Message}"; }
                                };
                                revisionCard.Children.Add(useCaption);
                            }
                            historyPanel.Children.Add(new Border {Padding=new Thickness(12),CornerRadius=new CornerRadius(10),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(28,120,100,80)),Child=revisionCard});
                        }
                        historyButton.Content="Hide caption history";
                        historyPanel.Visibility=Visibility.Visible;
                    } catch(Exception error) { Status.Text=$"Could not load caption history: {error.Message}"; }
                    finally { historyButton.IsEnabled=true; }
                };
                panel.Children.Add(historyButton);
                panel.Children.Add(historyPanel);
            } controls.Add(panel);
        } ContentItems.ItemsSource=controls;
    }
 private void RenderPhotoDuplicateReview(List<UIElement> controls,JsonNode room) {
  var roomId=room["id"]!.GetValue<string>();
  var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,4,0,16)};
  panel.Children.Add(new TextBlock {Text="Possible look-alikes",FontSize=22,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia")});
  panel.Children.Add(new TextBlock {Text="A private visual check. Compare each suggestion before making changes.",TextWrapping=TextWrapping.Wrap,Opacity=0.72});
  var scan=new Button {Content=scanningPhotoDuplicates?"Checking photos…":"Find similar photos",HorizontalAlignment=HorizontalAlignment.Left,IsEnabled=!scanningPhotoDuplicates};
  Microsoft.UI.Xaml.Automation.AutomationProperties.SetAutomationId(scan,"library.similar_photos.scan");
  scan.Click+=async(_,_)=>{
   if(core==null) return;
   scanningPhotoDuplicates=true;
   Render();
   try {
    photoDuplicateResponse=await core.Command(new {action="suggest_photo_duplicates",room_id=roomId});
    photoDuplicateRoomId=roomId;
   } catch(Exception e) { Status.Text=e.Message; }
   finally { scanningPhotoDuplicates=false; Render(); }
  };
  panel.Children.Add(scan);
  if(photoDuplicateRoomId==roomId && photoDuplicateResponse!=null) {
   var response=photoDuplicateResponse;
   var indexed=response["indexed_photos"]?.GetValue<int>()??0;
   var unavailable=response["unavailable_locally"]?.GetValue<int>()??0;
   var skipped=response["unsupported_or_oversized"]?.GetValue<int>()??0;
   panel.Children.Add(new TextBlock {Text=$"{indexed} checked on this device · {unavailable} originals unavailable · {skipped} skipped",Opacity=0.72,TextWrapping=TextWrapping.Wrap});
   var suggestions=response["suggestions"] as JsonArray;
   if(suggestions==null || suggestions.Count==0) {
    panel.Children.Add(new TextBlock {Text="No close visual matches were found in the photos checked.",Opacity=0.72,TextWrapping=TextWrapping.Wrap});
   } else {
    foreach(var pair in suggestions.OfType<JsonObject>()) {
     var card=new StackPanel {Spacing=8,Padding=new Thickness(12),Margin=new Thickness(0,4,0,4),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(30,120,100,80))};
     card.Children.Add(new TextBlock {Text="Compare these two photos",FontWeight=Microsoft.UI.Text.FontWeights.SemiBold});
     AddPhotoDuplicateMember(card,room,pair["first"]);
     AddPhotoDuplicateMember(card,room,pair["second"]);
     panel.Children.Add(card);
    }
    if(response["comparison_limit_reached"]?.GetValue<bool>()==true || response["result_limit_reached"]?.GetValue<bool>()==true)
     panel.Children.Add(new TextBlock {Text="Showing a bounded set of matches. Review these before checking again.",Opacity=0.72,TextWrapping=TextWrapping.Wrap});
   }
  }
  controls.Add(panel);
 }
 private void AddPhotoDuplicateMember(StackPanel panel,JsonNode room,JsonNode? member) {
  var assetId=member?["asset_id"]?.GetValue<string>();
  var asset=(room["assets"]?.AsArray()??new JsonArray()).FirstOrDefault(item=>item?["id"]?.GetValue<string>()==assetId && item?["deleted"]?.GetValue<bool>()!=true);
  if(asset==null) return;
  var title=asset["caption"]?.GetValue<string>();
  if(string.IsNullOrWhiteSpace(title)) title=asset["filename"]?.GetValue<string>()??"Photo";
  var row=new StackPanel {Orientation=Orientation.Horizontal,Spacing=12};
  row.Children.Add(new Border {Width=88,Height=68,CornerRadius=new CornerRadius(10),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(255,233,228,222)),Child=new FontIcon {Glyph="",FontSize=24}});
  row.Children.Add(new TextBlock {Text=title,VerticalAlignment=VerticalAlignment.Center,TextWrapping=TextWrapping.Wrap,Width=180});
  var open=new Button {Content="Review photo",VerticalAlignment=VerticalAlignment.Center};
  var copy=asset.DeepClone();
  var roomId=room["id"]!.GetValue<string>();
  open.Click+=async(_,_)=>await OpenOriginal(copy,roomId);
  row.Children.Add(open);
  panel.Children.Add(row);
 }
 private void RenderMomentSuggestions(List<UIElement> controls,JsonNode room) {
        var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,4,0,16)};
        panel.Children.Add(new TextBlock {Text="Possible Moments",FontSize=22,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia")});
        panel.Children.Add(new TextBlock {Text="Capture-time groupings stay suggestions until you review and save them.",TextWrapping=TextWrapping.Wrap,Opacity=0.72});
        var roomId=room["id"]!.GetValue<string>();
        var find=new Button {Content=momentSuggestionsRoomId==roomId?"Refresh suggestions":"Find possible Moments",HorizontalAlignment=HorizontalAlignment.Left};
        find.Click+=async(_,_)=>{
            if(core==null) return;
            find.IsEnabled=false;
            try {
                var result=await core.Command(new {action="suggest_moments",room_id=roomId});
                momentSuggestions=result["suggestions"]?.AsArray() is JsonArray suggestions
                    ? (JsonArray)suggestions.DeepClone() : new JsonArray();
                momentSuggestionsRoomId=roomId;
                Render();
            } catch(Exception e) { Status.Text=e.Message; }
            finally { find.IsEnabled=true; }
        };
        panel.Children.Add(find);
        if(momentSuggestionsRoomId==roomId && momentSuggestions!=null) {
            if(momentSuggestions.Count==0) panel.Children.Add(new TextBlock {Text="No new capture-time groupings are ready to review.",Opacity=0.72,TextWrapping=TextWrapping.Wrap});
            foreach(var suggestion in momentSuggestions.OfType<JsonObject>()) {
                var ids=suggestion["asset_ids"]?.AsArray().Select(id=>id?.GetValue<string>()??"").Where(id=>id.Length>0).ToHashSet(StringComparer.Ordinal)??new HashSet<string>(StringComparer.Ordinal);
                var count=ids.Count(id=>room["assets"]?.AsArray().Any(asset=>asset?["id"]?.GetValue<string>()==id && asset["deleted"]?.GetValue<bool>()!=true)==true);
                var timestamp=suggestion["start"]?.GetValue<long>()??0;
                var date=DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().ToString("MMMM d, yyyy");
                var card=new StackPanel {Spacing=6,Padding=new Thickness(14),Margin=new Thickness(0,4,0,4),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(28,120,100,80))};
                card.Children.Add(new TextBlock {Text=date,FontSize=17,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold});
                card.Children.Add(new TextBlock {Text=$"{count} photos and videos",Opacity=0.72});
                var review=new Button {Content="Review selection",HorizontalAlignment=HorizontalAlignment.Left,IsEnabled=count>0};
                var roomCopy=room.DeepClone(); var suggestionCopy=suggestion.DeepClone();
                review.Click+=async(_,_)=>await ReviewMomentSuggestion(roomCopy,suggestionCopy);
                card.Children.Add(review); panel.Children.Add(card);
            }
        }
        controls.Add(panel);
    }
    private async Task ReviewMomentSuggestion(JsonNode room,JsonNode suggestion) {
        var roomId=room["id"]!.GetValue<string>();
        var selectedIds=(suggestion["asset_ids"]?.AsArray()??new JsonArray())
            .Select(id=>id?.GetValue<string>()??"").Where(id=>id.Length>0).ToHashSet(StringComparer.Ordinal);
        var editor=new StackPanel {Spacing=10,Padding=new Thickness(4)};
        editor.Children.Add(new TextBlock {Text="This draft was grouped by capture time. Adjust its title and selection before adding it to the Room.",TextWrapping=TextWrapping.Wrap});
        var timestamp=suggestion["start"]?.GetValue<long>()??0;
        var date=DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().ToString("MMMM d, yyyy");
        var title=new TextBox {Header="Moment title",Text=$"Moment · {date}"}; editor.Children.Add(title);
        var featured=new CheckBox {Content="Feature on Home"}; editor.Children.Add(featured);
        var assetChecks=new List<(string Id,CheckBox Control)>();
        var assetList=new StackPanel {Spacing=4};
        foreach(var asset in room["assets"]?.AsArray()??new JsonArray()) {
            var id=asset?["id"]?.GetValue<string>();
            if(id==null || !selectedIds.Contains(id) || asset?["deleted"]?.GetValue<bool>()==true) continue;
            var label=asset?["caption"]?.GetValue<string>();
            if(string.IsNullOrWhiteSpace(label)) label=asset?["filename"]?.GetValue<string>()??"Memory";
            var check=new CheckBox {Content=label,IsChecked=true}; assetChecks.Add((id,check)); assetList.Children.Add(check);
        }
        var scroll=new ScrollViewer {Content=assetList,MaxHeight=360}; editor.Children.Add(scroll);
        var dialog=new ContentDialog {Title="Review Moment",Content=editor,PrimaryButtonText="Save Moment",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot};
        if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
        var chosen=assetChecks.Where(item=>item.Control.IsChecked==true).Select(item=>item.Id).ToArray();
        if(chosen.Length==0) { Status.Text="Choose at least one memory before saving the Moment."; return; }
        try {
            await core!.Command(new {action="save_moment",room_id=roomId,title=title.Text.Trim(),featured=featured.IsChecked==true,asset_ids=chosen});
            momentSuggestions=null; momentSuggestionsRoomId="";
            await Reload(); Status.Text="Moment saved to this Room.";
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private static bool CanEditAssets(JsonNode room) => room["role"]?.GetValue<string>() is "owner" or "admin" or "contributor";
    private void RenderPersonMerge(List<UIElement> controls,JsonNode room) {
        if(!CanEditAssets(room)) return;
        var people=(room["people"]?.AsArray()??new JsonArray())
            .Where(person=>person?["id"]!=null)
            .Select(person=>new {Id=person!["id"]!.GetValue<string>(),Name=person["name"]?.GetValue<string>()??"Person"})
            .ToList();
        if(people.Count<2) return;
        var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,4,0,16)};
        panel.Children.Add(new TextBlock {Text="Merge duplicate People",FontSize=20,FontFamily="Georgia"});
        panel.Children.Add(new TextBlock {Text="Use this when two profiles show the same person. This changes People labels and tags only; it never invites anyone or grants Room access.",TextWrapping=TextWrapping.Wrap,Opacity=0.72});
        var source=new ComboBox {Header="Profile to merge",MinWidth=240,SelectedIndex=0};
        var target=new ComboBox {Header="Keep this profile",MinWidth=240,SelectedIndex=1};
        foreach(var person in people) {
            source.Items.Add(new ComboBoxItem {Content=person.Name,Tag=person.Id});
            target.Items.Add(new ComboBoxItem {Content=person.Name,Tag=person.Id});
        }
        panel.Children.Add(source);
        panel.Children.Add(target);
        var merge=new Button {Content="Merge profiles…",HorizontalAlignment=HorizontalAlignment.Left};
        merge.Click+=async(_,_)=>{
            if(source.SelectedItem is not ComboBoxItem from || from.Tag is not string sourceId ||
               target.SelectedItem is not ComboBoxItem into || into.Tag is not string targetId) return;
            if(sourceId==targetId) { Status.Text="Choose two different profiles."; return; }
            var roomId=room["id"]!.GetValue<string>();
            var dialog=new ContentDialog {
                Title=$"Merge {from.Content} into {into.Content}?",
                Content="The kept profile will contain both sets of photo tags. This correction applies across this Room.",
                PrimaryButtonText="Merge profiles",CloseButtonText="Cancel",
                DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot
            };
            if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
            await ApplyLibraryBatch(new {action="merge_person",room_id=roomId,source_person_id=sourceId,target_person_id=targetId});
            if(string.Equals(Status.Text,"Updated selected memories.",StringComparison.Ordinal)) Status.Text="People merged across this Room.";
        };
        panel.Children.Add(merge);
        controls.Add(panel);
    }

    private List<string> SelectedLibraryAssetIds(JsonNode room,bool includeDeleted=false) => room["assets"]?.AsArray()
        .Where(asset=>(asset?["deleted"]?.GetValue<bool>()==true)==includeDeleted)
        .Where(asset=>selectedAssets.Contains(asset?["id"]?.GetValue<string>()??""))
        .Select(asset=>asset!["id"]!.GetValue<string>()).ToList() ?? new List<string>();
    private bool CanChangeDeletionForSelection(JsonNode room,List<string> ids) {
        if(ids.Count==0) return false;
        var role=room["role"]?.GetValue<string>();
        if(role is "owner" or "admin") return true;
        if(role!="contributor") return false;
        var deviceId=snapshot?["device_id"]?.GetValue<string>();
        return deviceId!=null && ids.All(id=>room["assets"]?.AsArray()
            .Any(asset=>asset?["id"]?.GetValue<string>()==id && asset["contributor"]?.GetValue<string>()==deviceId)==true);
    }
    private static long CorrectCaptureTimestamp(long originalTimestamp,DateTimeOffset targetDate) {
        var original=DateTimeOffset.FromUnixTimeSeconds(originalTimestamp).ToLocalTime();
        var correctedLocal=new DateTime(targetDate.Year,targetDate.Month,targetDate.Day,
            original.Hour,original.Minute,original.Second,DateTimeKind.Local);
        return new DateTimeOffset(correctedLocal).ToUnixTimeSeconds();
    }
    private void SetAssetSelected(string assetId,bool selected) {
        if(selected) { if(selectedAssets.Count<MaxBulkSelection) selectedAssets.Add(assetId); }
        else selectedAssets.Remove(assetId);
        DispatcherQueue.TryEnqueue(Render);
    }
    private async Task ApplyLibraryBatch(object request) {
        if(core==null) return;
        try {
            await core.Command(request);
            selectedAssets.Clear(); selectingAssets=false;
            await Reload(); Status.Text="Updated selected memories.";
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private void RenderLibrarySelection(List<UIElement> controls,JsonNode room) {
        var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,4,0,16)};
        var canEdit=CanEditAssets(room);
        var isTrash=section=="trash";
        var selected=SelectedLibraryAssetIds(room,isTrash);
        if(!selectingAssets) {
            var start=new Button {Content=isTrash ? "Select deleted memories" : "Select memories",IsEnabled=canEdit,HorizontalAlignment=HorizontalAlignment.Left};
            start.Click+=(_,_)=>{selectingAssets=true;Render();};
            panel.Children.Add(start); controls.Add(panel); return;
        }
        var selectionRow=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
        selectionRow.Children.Add(new TextBlock {Text=$"{selected.Count} selected"+(selected.Count==MaxBulkSelection?" · maximum per batch":""),VerticalAlignment=VerticalAlignment.Center});
        var all=new Button {Content="Select up to 500",IsEnabled=canEdit};
        all.Click+=(_,_)=>{selectedAssets.Clear();foreach(var asset in room["assets"]?.AsArray()??new JsonArray()) { if((asset?["deleted"]?.GetValue<bool>()==true)!=isTrash) continue; var id=asset?["id"]?.GetValue<string>(); if(id!=null && selectedAssets.Count<MaxBulkSelection) selectedAssets.Add(id); }Render();};
        selectionRow.Children.Add(all);
        var clear=new Button {Content="Clear",IsEnabled=selected.Count>0};
        clear.Click+=(_,_)=>{selectedAssets.Clear();Render();}; selectionRow.Children.Add(clear);
        var done=new Button {Content="Done"}; done.Click+=(_,_)=>{selectedAssets.Clear();selectingAssets=false;Render();}; selectionRow.Children.Add(done);
        panel.Children.Add(selectionRow);
        if(selected.Count>0 && isTrash) {
            var restore=new Button {Content="Restore selected",IsEnabled=CanChangeDeletionForSelection(room,selected)};
            restore.Click+=async(_,_)=>await ApplyLibraryBatch(new {action="patch_assets",room_id=room["id"]!.GetValue<string>(),asset_ids=selected.ToArray(),fields=new {deleted=false}});
            panel.Children.Add(restore);
        }
        if(selected.Count>0 && !isTrash) {
            var favoriteValue=!selected.All(id=>room["assets"]!.AsArray().Any(asset=>asset?["id"]?.GetValue<string>()==id && asset["favorite"]?.GetValue<bool>()==true));
            var edits=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            var favorite=new Button {Content=favoriteValue?"Favorite":"Unfavorite",IsEnabled=canEdit};
            var roomId=room["id"]!.GetValue<string>(); var ids=selected.ToArray();
            favorite.Click+=async(_,_)=>await ApplyLibraryBatch(new {action="patch_assets",room_id=roomId,asset_ids=ids,fields=new {favorite=favoriteValue}});
            edits.Children.Add(favorite);
            var correctDate=new Button {Content="Correct date",IsEnabled=canEdit};
            correctDate.Click+=async(_,_)=>{
                var firstTimestamp=room["assets"]?.AsArray()
                    .FirstOrDefault(asset=>asset?["id"]?.GetValue<string>()==ids[0])?["captured_at"]?.GetValue<long>() ?? DateTimeOffset.Now.ToUnixTimeSeconds();
                var datePicker=new DatePicker {Header="New calendar date",Date=DateTimeOffset.FromUnixTimeSeconds(firstTimestamp).ToLocalTime()};
                var content=new StackPanel {Spacing=8};
                content.Children.Add(new TextBlock {Text="Each memory keeps its original local time of day and ordering."});
                content.Children.Add(datePicker);
                var dialog=new ContentDialog {Title="Correct capture date",Content=content,PrimaryButtonText=$"Apply to {ids.Length} memories",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot};
                if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
                try {
                    var dates=ids.Select(assetId=>{
                        var asset=room["assets"]?.AsArray().FirstOrDefault(item=>item?["id"]?.GetValue<string>()==assetId);
                        var capturedAt=asset?["captured_at"]?.GetValue<long>() ?? throw new InvalidOperationException("A selected memory has no capture date.");
                        return new {asset_id=assetId,captured_at=CorrectCaptureTimestamp(capturedAt,datePicker.Date)};
                    }).ToArray();
                    await ApplyLibraryBatch(new {action="patch_asset_dates",room_id=roomId,dates});
                } catch(Exception e) { Status.Text=e.Message; }
            };
            edits.Children.Add(correctDate);
            var people=(room["people"]?.AsArray()??new JsonArray()).Where(person=>person?["id"]!=null)
                .Select(person=>new {Id=person!["id"]!.GetValue<string>(),Name=person["name"]?.GetValue<string>()??"Person"}).ToList();
            var personPicker=new ComboBox {PlaceholderText="Choose Person",MinWidth=160,IsEnabled=canEdit&&people.Count>0};
            foreach(var person in people) personPicker.Items.Add(new ComboBoxItem {Content=person.Name,Tag=person.Id});
            edits.Children.Add(personPicker);
            foreach(var (label,remove) in new[]{("Add Person",false),("Remove Person",true)}) {
                var tag=new Button {Content=label,IsEnabled=canEdit&&people.Count>0};
                tag.Click+=async(_,_)=>{if(personPicker.SelectedItem is not ComboBoxItem choice || choice.Tag is not string personId) return; await ApplyLibraryBatch(new {action="tag_assets",room_id=roomId,person_id=personId,asset_ids=ids,remove});};
                edits.Children.Add(tag);
            }
            panel.Children.Add(edits);
            var albumRow=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            var createAlbum=new Button {Content="Create album",IsEnabled=canEdit};
            createAlbum.Click+=async(_,_)=>{
                var title=new TextBox {PlaceholderText="Album name"};
                var dialog=new ContentDialog {Title="Create album from selection",Content=title,PrimaryButtonText="Create",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot};
                if(await dialog.ShowAsync()==ContentDialogResult.Primary && !string.IsNullOrWhiteSpace(title.Text))
                    await ApplyLibraryBatch(new {action="save_album",room_id=roomId,title=title.Text.Trim(),asset_ids=ids});
            };
            albumRow.Children.Add(createAlbum); panel.Children.Add(albumRow);
            var mayTrash=CanChangeDeletionForSelection(room,selected);
            var moveToTrash=new Button {Content="Move to Recently Deleted",IsEnabled=mayTrash,Foreground=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.DarkRed)};
            moveToTrash.Click+=async(_,_)=>{
                var dialog=new ContentDialog {Title="Move memories to Recently Deleted?",Content="The originals stay saved. You can restore these memories later.",PrimaryButtonText=$"Move {selected.Count} memories",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot};
                if(await dialog.ShowAsync()==ContentDialogResult.Primary)
                    await ApplyLibraryBatch(new {action="patch_assets",room_id=roomId,asset_ids=ids,fields=new {deleted=true}});
            };
            var deleteRow=new StackPanel {Spacing=6};
            deleteRow.Children.Add(moveToTrash);
            if(!mayTrash) deleteRow.Children.Add(new TextBlock {Text="Select only memories you contributed, or ask a Room owner or admin.",Opacity=0.72,TextWrapping=TextWrapping.Wrap});
            panel.Children.Add(deleteRow);
        }
        controls.Add(panel);
    }
    private void RenderStorageSources() {
        if(StorageSources==null) return;
        var controls=new List<UIElement>();
        var roomId=Room?["id"]?.GetValue<string>();
        if(roomId==null || snapshot?["sources"] is not JsonArray sources) { StorageSources.ItemsSource=controls; return; }
        var cleanup=new StackPanel {Spacing=6,Margin=new Thickness(0,4,0,12)};
        cleanup.Children.Add(new TextBlock {Text="Free device space",FontSize=16});
        cleanup.Children.Add(new TextBlock {Text="Only originals with another verified copy are eligible. Offline pins stay here; older unpinned memories are considered first.",Opacity=0.75,TextWrapping=TextWrapping.Wrap});
        var cleanupActions=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
        foreach(var (label,bytes) in new[]{("500 MB",500L*1024*1024),("1 GB",1024L*1024*1024),("5 GB",5L*1024*1024*1024)}) {
            var free=new Button {Content=label};
            free.Click+=async(_,_)=>await FreeDeviceStorage(roomId,bytes);
            cleanupActions.Children.Add(free);
        }
        cleanup.Children.Add(cleanupActions);
        controls.Add(cleanup);
        long? cacheLimit=null;
        if(Room?["device_cache_limit_bytes"] is JsonValue cacheLimitValue && cacheLimitValue.TryGetValue<long>(out var parsedCacheLimit)) cacheLimit=parsedCacheLimit;
        var policy=new StackPanel {Spacing=6,Margin=new Thickness(0,0,0,12)};
        policy.Children.Add(new TextBlock {Text=$"Automatic device cache · {(cacheLimit.HasValue ? FormatStorageBytes(cacheLimit.Value) : "No limit")}",FontSize=16});
        policy.Children.Add(new TextBlock {Text="After Room sync, Family Room removes only originals with another verified copy. Offline pins and sole copies stay here.",Opacity=0.75,TextWrapping=TextWrapping.Wrap});
        var policyActions=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
        foreach(var (label,limit) in new (string Label,long? Limit)[]{("No limit",null),("2 GB",2_147_483_648L),("5 GB",5_368_709_120L),("10 GB",10_737_418_240L)}) {
            var choose=new Button {Content=cacheLimit==limit ? $"✓ {label}" : label};
            choose.Click+=async(_,_)=>await SetDeviceCacheLimit(roomId,limit);
            policyActions.Children.Add(choose);
        }
        policy.Children.Add(policyActions);
        controls.Add(policy);
        foreach(var source in sources) {
            if(source?["room_id"]?.GetValue<string>()!=roomId) continue;
            var id=source["id"]!.GetValue<string>();
            var preferred=source["preferred"]?.GetValue<bool>()==true;
            var overflow=source["overflow"]?.GetValue<bool>()==true;
            var panel=new StackPanel {Spacing=5,Margin=new Thickness(0,4,0,10)};
            panel.Children.Add(new TextBlock {Text=source["name"]?.GetValue<string>()??"Storage source",FontSize=16});
            panel.Children.Add(new TextBlock {Text=preferred ? "Preferred for new uploads" : overflow ? "Used when preferred storage is full" : source["kind"]?.GetValue<string>()??"",Opacity=0.75});
            if(source["kind"]?.GetValue<string>()!="local") {
                var unmetered=source["network_policy"]?.GetValue<string>()=="unmetered_only";
                var networkPolicy=new Button {Content=unmetered ? "Media: unmetered only" : "Media: any network"};
                networkPolicy.Click+=async(_,_)=>{await Execute(new {action="set_source_network_policy",id,policy=unmetered ? "any" : "unmetered_only"});if(unmetered) await Sync();};
                panel.Children.Add(networkPolicy);
            }
            var actions=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            if(!preferred) {
                var makePreferred=new Button {Content="Make preferred"};
                makePreferred.Click+=async(_,_)=>{await Execute(new {action="prefer_source",id});await Sync();};
                actions.Children.Add(makePreferred);
                var chooseOverflow=new Button {Content=overflow ? "Clear overflow" : "Use for overflow"};
                chooseOverflow.Click+=async(_,_)=>{await Execute(new {action="set_overflow",room_id=roomId,id=overflow ? null : id});await Sync();};
                actions.Children.Add(chooseOverflow);
            }
            panel.Children.Add(actions);
            controls.Add(panel);
        }
        StorageSources.ItemsSource=controls;
    }
    private static string FormatStorageBytes(long bytes) => bytes>=1_000_000_000L ? $"{bytes/1_000_000_000d:0.0} GB" : bytes>=1_000_000L ? $"{bytes/1_000_000d:0} MB" : $"{bytes} bytes";
    private async Task FreeDeviceStorage(string roomId,long bytes) {
        if(core==null) return;
        var dialog=new ContentDialog {
            Title="Free device storage?",
            Content=$"Family Room will remove local originals only after verifying another copy. Pinned originals stay here. Try to free {FormatStorageBytes(bytes)}?",
            PrimaryButtonText="Free space",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot
        };
        if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
        try {
            var result=await core.Command(new {action="free_device_storage",room_id=roomId,bytes_to_free=bytes});
            await Reload();
            var freed=result["bytes_freed"]?.GetValue<long>()??0;
            var complete=result["complete"]?.GetValue<bool>()==true;
            var pinned=result["pinned_originals_preserved"]?.GetValue<int>()??0;
            Status.Text=$"Freed {FormatStorageBytes(freed)}. {(complete ? "Request met." : "Not enough verified copies to meet the full request.")} {pinned} pinned originals stayed here.";
        } catch(Exception error) { Status.Text=error.Message; }
    }
    private async Task SetDeviceCacheLimit(string roomId,long? maxBytes) {
        if(core==null) return;
        if(maxBytes.HasValue) {
            var dialog=new ContentDialog {
                Title="Set an automatic cache limit?",
                Content=$"After Room sync, Family Room will remove local originals above {FormatStorageBytes(maxBytes.Value)} only after verifying another copy. Offline pins and sole copies stay here.",
                PrimaryButtonText="Set limit",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot
            };
            if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
        }
        try {
            var result=await core.Command(new {action="set_device_cache_limit",room_id=roomId,max_bytes=maxBytes});
            await Reload();
            var cleanup=result["cleanup"];
            var freed=cleanup?["bytes_freed"]?.GetValue<long>()??0;
            var complete=cleanup?["complete"]?.GetValue<bool>()??true;
            Status.Text=maxBytes.HasValue
                ? $"Cache limit set to {FormatStorageBytes(maxBytes.Value)}. Freed {FormatStorageBytes(freed)} now. {(complete ? "Limit met." : "Cleanup will retry after another verified copy is available.")}"
                : "Automatic cache cleanup is off.";
        } catch(Exception error) { Status.Text=error.Message; }
    }
    private void RenderMembers() {
        if(MembersPanel==null) return;
        var room=Room;
        if(room==null || room["members"] is not JsonArray members) {
            MembersPanel.Visibility=Visibility.Collapsed;
            RoomMembers.ItemsSource=Array.Empty<UIElement>();
            OwnershipPreparePanel.Visibility=Visibility.Collapsed;
            OwnershipAcceptPanel.Visibility=Visibility.Collapsed;
            LeaveRoomPanel.Visibility=Visibility.Collapsed;
            return;
        }
        MembersPanel.Visibility=Visibility.Visible;
        MembersHeading.Text=$"Members of {room["name"]?.GetValue<string>()??"Room"}";
        var roomId=room["id"]!.GetValue<string>();
        var ownerId=room["owner_id"]?.GetValue<string>()??"";
        var canManage=room["role"]?.GetValue<string>()=="owner";
        var albumOnly=room["album_only"]?.GetValue<bool>()==true;
        var currentDevice=snapshot?["device_id"]?.GetValue<string>()??"";
        var eligible=members.Where(member=>member!=null
            && member["device_id"]?.GetValue<string>() is string id
            && id!=currentDevice && id!=ownerId).ToList();
        if(ownershipOfferRoomId!=roomId || !canManage || !eligible.Any(member=>
            member?["device_id"]?.GetValue<string>()==ownershipOfferDevice)) {
            ownershipOfferRoomId="";
            ownershipOfferDevice="";
            OwnershipOffer.Text="";
        }
        OwnershipPreparePanel.Visibility=canManage && eligible.Count>0 ? Visibility.Visible : Visibility.Collapsed;
        OwnershipAcceptPanel.Visibility=!canManage && room["role"]?.GetValue<string>()!="removed"
            ? Visibility.Visible : Visibility.Collapsed;
        LeaveRoomPanel.Visibility=canManage ? Visibility.Collapsed : Visibility.Visible;
        LeaveRoomNote.Text=canManage
            ? "Transfer ownership to another Member before leaving."
            : "This removes this device’s Room keys and storage connections. Ask an owner to remove this device from the shared roster to revoke its grant for the Room.";
        LeaveRoomButton.Content=room["role"]?.GetValue<string>()=="removed"
            ? "Forget this Room" : "Leave Room";
        var previousSelection=OwnershipSuccessor.SelectedValue?.ToString();
        OwnershipSuccessor.Items.Clear();
        foreach(var member in eligible) {
            var id=member!["device_id"]!.GetValue<string>();
            var role=member["role"]?.GetValue<string>()??"viewer";
            OwnershipSuccessor.Items.Add(new ComboBoxItem {Content=$"Device {id[..Math.Min(8,id.Length)]} · {role}",Tag=id});
        }
        if(OwnershipSuccessor.Items.Count>0) {
            var index=Enumerable.Range(0,OwnershipSuccessor.Items.Count)
                .FirstOrDefault(i=>(OwnershipSuccessor.Items[i] as ComboBoxItem)?.Tag?.ToString()==previousSelection,-1);
            OwnershipSuccessor.SelectedIndex=index>=0 ? index : 0;
        }
        var rows=new List<UIElement>();
        foreach(var member in members) {
            if(member==null) continue;
            var deviceId=member["device_id"]?.GetValue<string>()??"";
            if(deviceId.Length==0) continue;
            var role=member["role"]?.GetValue<string>()??"viewer";
            var isOwner=deviceId==ownerId;
            var content=new StackPanel {Spacing=8};
            content.Children.Add(new TextBlock {Text=isOwner?"Room owner":"Member device",FontSize=16,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold});
            var identity=new TextBlock {Text=$"{deviceId}\nRole: {role}",TextWrapping=TextWrapping.Wrap,IsTextSelectionEnabled=true,Opacity=0.78};
            content.Children.Add(identity);
            if(canManage && !isOwner) {
                var actions=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
                foreach(var newRole in albumOnly ? new[]{"viewer"} : new[]{"viewer","contributor","admin"}) {
                    var change=new Button {Content=newRole,IsEnabled=role!=newRole};
                    change.Click+=async(_,_)=>await ChangeMemberRole(roomId,deviceId,newRole);
                    actions.Children.Add(change);
                }
                content.Children.Add(actions);
                var remove=new Button {Content="Remove device…",HorizontalAlignment=HorizontalAlignment.Left};
                remove.Click+=async(_,_)=>await RevokeMemberDevice(roomId,deviceId);
                content.Children.Add(remove);
            } else if(isOwner) {
                content.Children.Add(new TextBlock {Text="Transfer ownership before removing the owner device.",TextWrapping=TextWrapping.Wrap,Opacity=0.72});
            }
            rows.Add(new Border {
                Child=content,
                Padding=new Thickness(14),
                Margin=new Thickness(0,4,0,4),
                BorderThickness=new Thickness(1),
                BorderBrush=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.LightGray),
                CornerRadius=new CornerRadius(14),
            });
        }
        if(rows.Count==0) rows.Add(new TextBlock {Text="No member devices are listed yet."});
        RoomMembers.ItemsSource=rows;
    }
    private async void PrepareOwnershipTransfer(object sender,RoutedEventArgs args) {
        if(core==null || Room is not JsonNode room || OwnershipSuccessor.SelectedItem is not ComboBoxItem selected
            || selected.Tag is not string targetDevice) return;
        var roomId=room["id"]!.GetValue<string>();
        try {
            var result=await core.Command(new {action="prepare_owner_transfer",room_id=roomId,device_id=targetDevice});
            var offer=result["offer"]?.GetValue<string>()??throw new InvalidOperationException("The core returned no ownership offer.");
            OwnershipOffer.Text=offer;
            ownershipOfferRoomId=roomId;
            ownershipOfferDevice=targetDevice;
            Status.Text=$"Signed ownership offer prepared for device {targetDevice[..Math.Min(8,targetDevice.Length)]}. Share it privately.";
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private void CopyOwnershipOffer(object sender,RoutedEventArgs args) {
        if(string.IsNullOrWhiteSpace(OwnershipOffer.Text)) { Status.Text="Prepare an ownership offer first."; return; }
        var package=new DataPackage();
        package.SetText(OwnershipOffer.Text);
        Clipboard.SetContent(package);
        Status.Text="Signed ownership offer copied. Share it privately with the selected device.";
    }
    private async void ReviewOwnershipOffer(object sender,RoutedEventArgs args) {
        if(core==null || Room is not JsonNode room) return;
        if(string.IsNullOrWhiteSpace(OwnershipOfferInput.Text)) { Status.Text="Paste the signed offer before reviewing it."; return; }
        var dialog=new ContentDialog {
            Title="Accept Room ownership?",
            Content="This device becomes the Room owner. The current owner becomes an admin (or viewer for an album-only Room). Future contributions use a rotated key; previously downloaded copies remain with their Members.",
            PrimaryButtonText="Accept ownership",
            CloseButtonText="Cancel",
            DefaultButton=ContentDialogButton.None,
            XamlRoot=MembersPanel.XamlRoot,
        };
        if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
        var roomId=room["id"]!.GetValue<string>();
        try {
            await core.Command(new {action="accept_owner_transfer",room_id=roomId,offer=OwnershipOfferInput.Text.Trim()});
            OwnershipOfferInput.Text="";
            await Reload();
            var sources=snapshot?["sources"]?.AsArray().Where(source=>source?["room_id"]?.GetValue<string>()==roomId
                && source["paused"]?.GetValue<bool>()!=true).ToList()??new List<JsonNode?>();
            var synchronized=false;
            foreach(var source in sources) {
                try {
                    await core.Command(new {action="sync_source",id=source!["id"]!.GetValue<string>()});
                    synchronized=true;
                } catch { }
            }
            await Reload();
            Status.Text=synchronized
                ? "Ownership transferred and the signed update was sent to connected storage."
                : "Ownership transferred on this device. The signed update will publish when storage reconnects.";
        } catch(Exception e) { Status.Text=e.Message; }
    }

    private async void LeaveRoomClicked(object sender,RoutedEventArgs args) {
        var room=Room;
        if(core==null || room==null) return;
        var role=room["role"]?.GetValue<string>()??"viewer";
        if(role=="owner") {
            Status.Text="Transfer ownership to another Member before leaving.";
            return;
        }
        var roomId=room["id"]!.GetValue<string>();
        var forgotten=role=="removed";
        var dialog=new ContentDialog {
            Title=forgotten ? "Forget this Room?" : "Leave this Room on this device?",
            Content="This device will forget its Room keys and stop syncing this Room. Ask an owner to remove this device from the shared roster to revoke its grant. Other Members keep their copies; a saved device backup may restore this Room.",
            PrimaryButtonText=forgotten ? "Forget Room" : "Leave Room",
            CloseButtonText="Cancel",
            DefaultButton=ContentDialogButton.None,
            XamlRoot=MembersPanel.XamlRoot,
        };
        if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
        try {
            await core.Command(new {action="leave_room",room_id=roomId});
            await Reload();
            Status.Text="Room removed from this device. The owner’s member roster was not changed.";
        } catch(Exception e) { Status.Text=e.Message; }
    }

    private async Task ChangeMemberRole(string roomId,string deviceId,string role) {
        if(core==null) return;
        try {
            await core.Command(new {action="set_role",room_id=roomId,device_id=deviceId,role});
            await Sync();
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private async Task RevokeMemberDevice(string roomId,string deviceId) {
        if(core==null) return;
        var dialog=new ContentDialog {
            Title="Remove this device?",
            Content="The Room keys will rotate for future content. Copies already downloaded to that device cannot be recalled.",
            PrimaryButtonText="Remove access",
            CloseButtonText="Keep device",
            DefaultButton=ContentDialogButton.None,
            XamlRoot=MembersPanel.XamlRoot,
        };
        if(await dialog.ShowAsync()!=ContentDialogResult.Primary) return;
        try {
            await core.Command(new {action="revoke_device",room_id=roomId,device_id=deviceId});
            await Sync();
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private async Task<string> MaterializeOriginal(JsonNode asset,string roomId) {
            if(core==null) throw new InvalidOperationException("The Room is still opening.");
            var cache=Path.Combine(Path.GetTempPath(),"FamilyRoom",Guid.NewGuid().ToString());
            Directory.CreateDirectory(cache);
            var path=Path.Combine(cache,Path.GetFileName(asset["filename"]?.GetValue<string>()??"movie.mp4"));
            var id=asset["id"]!.GetValue<string>();
            if(asset["local"]?.GetValue<bool>()!=true) { await UpdateNetworkState(); await core.Command(new {action="fetch",room_id=roomId,asset_id=id}); }
            await core.Command(new {action="materialize",room_id=roomId,asset_id=id,destination=path});
            return path;
    }
    private async Task ExportVideoTranscript(JsonNode asset,string roomId) {
        if(core==null) return;
        try {
            var result=await core.Command(new {action="export_video_transcript",room_id=roomId,asset_id=asset["id"]!.GetValue<string>()});
            var cueCount=result["cue_count"]?.GetValue<int>()??0;
            var webvtt=result["webvtt"]?.GetValue<string>()??"";
            if(cueCount==0 || string.IsNullOrWhiteSpace(webvtt)) { Status.Text="No spoken captions are indexed yet. Use Ask your videos first."; return; }
            var picker=new FileSavePicker {SuggestedFileName=Path.GetFileNameWithoutExtension(asset["filename"]?.GetValue<string>()??"video")+"-captions"};
            picker.FileTypeChoices.Add("WebVTT captions",new List<string>{".vtt"});
            WinRT.Interop.InitializeWithWindow.Initialize(picker,this);
            var file=await picker.PickSaveFileAsync();
            if(file==null) return;
            await Windows.Storage.FileIO.WriteTextAsync(file,webvtt);
            Status.Text=$"Saved {cueCount} recognized speech captions. Review wording against the original.";
        } catch(Exception error) { Status.Text=error.Message; }
    }

    private async Task OpenOriginal(JsonNode asset,string roomId) {
        try {
            var path=await MaterializeOriginal(asset,roomId);
            await Windows.System.Launcher.LaunchFileAsync(await Windows.Storage.StorageFile.GetFileFromPathAsync(path));
        } catch(Exception e) {Status.Text=e.Message;}
    }
    private void RenderOnThisDay(List<UIElement> controls,JsonNode room) {
        var ids=new HashSet<string>((room["on_this_day_asset_ids"]?.AsArray()??new JsonArray())
            .Where(id=>id!=null).Select(id=>id!.GetValue<string>()),StringComparer.Ordinal);
        var assets=(room["assets"]?.AsArray()??new JsonArray())
            .Where(asset=>asset!=null && ids.Contains(asset["id"]?.GetValue<string>()??""))
            .OrderByDescending(asset=>asset!["captured_at"]?.GetValue<long>()??0)
            .Take(8).ToList();
        if(assets.Count==0) return;
        var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,4,0,18)};
        panel.Children.Add(new TextBlock {Text="On this day",FontSize=26,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia")});
        panel.Children.Add(new TextBlock {Text="Moments from earlier years",Opacity=0.72});
        var roomId=room["id"]!.GetValue<string>();
        foreach(var candidate in assets) {
            var asset=candidate!;
            var timestamp=asset["captured_at"]?.GetValue<long>()??0;
            var date=DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().ToString("MMMM d, yyyy");
            var title=asset["caption"]?.GetValue<string>() is string caption && caption.Length>0
                ? caption : asset["filename"]?.GetValue<string>()??"Memory";
            var content=new StackPanel {Spacing=3};
            content.Children.Add(new TextBlock {Text=date,Opacity=0.72});
            content.Children.Add(new TextBlock {Text=title,TextWrapping=TextWrapping.Wrap});
            var open=new Button {Content=content,HorizontalAlignment=HorizontalAlignment.Stretch,HorizontalContentAlignment=HorizontalAlignment.Left};
            var selected=asset.DeepClone();
            open.Click+=async(_,_)=>await OpenOriginal(selected,roomId);
            panel.Children.Add(open);
        }
        controls.Add(panel);
    }
    private void RenderMomentRiver(List<UIElement> controls,JsonNode room) {
        var roomId=room["id"]!.GetValue<string>();
        if(momentRiverRoomId!=roomId) { momentRiverRoomId=roomId; momentRiverYear=""; }
        var moments=(room["moments"]?.AsArray()??new JsonArray())
            .OfType<JsonObject>()
            .OrderBy(moment=>moment["start"]?.GetValue<long>()??0)
            .ThenBy(moment=>moment["id"]?.GetValue<string>(),StringComparer.Ordinal)
            .ToList();
        if(moments.Count==0) return;
        var years=moments.Select(moment=>MomentYear(moment["start"]?.GetValue<long>()??0))
            .Where(year=>year.Length>0).Distinct().OrderByDescending(year=>year,StringComparer.Ordinal).ToList();
        if(!years.Contains(momentRiverYear,StringComparer.Ordinal)) momentRiverYear="";
        var visible=string.IsNullOrEmpty(momentRiverYear) ? moments : moments
            .Where(moment=>MomentYear(moment["start"]?.GetValue<long>()??0)==momentRiverYear).ToList();
        var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,4,0,18)};
        panel.Children.Add(new TextBlock {Text="Memory river",FontSize=26,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia")});
        panel.Children.Add(new TextBlock {Text="Move through your Room’s Moments in date order. Cards grow with the memories inside.",TextWrapping=TextWrapping.Wrap,Opacity=0.72});
        var yearStrip=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
        void AddYear(string label,string value) {
            var chip=new Button {Content=label,Tag=value,Padding=new Thickness(14,7,14,7),
                Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(value==momentRiverYear
                    ? Microsoft.UI.ColorHelper.FromArgb(255,138,77,50)
                    : Microsoft.UI.ColorHelper.FromArgb(255,240,234,227)),
                Foreground=new Microsoft.UI.Xaml.Media.SolidColorBrush(value==momentRiverYear
                    ? Microsoft.UI.Colors.White
                    : Microsoft.UI.ColorHelper.FromArgb(255,112,102,95))};
            chip.Click+=(_,_)=>{ momentRiverYear=value; Render(); };
            yearStrip.Children.Add(chip);
        }
        AddYear("All years","");
        foreach(var year in years) AddYear(year,year);
        panel.Children.Add(new ScrollViewer {Content=yearStrip,HorizontalScrollMode=ScrollMode.Enabled,
            HorizontalScrollBarVisibility=ScrollBarVisibility.Auto,VerticalScrollMode=ScrollMode.Disabled,
            VerticalScrollBarVisibility=ScrollBarVisibility.Disabled,Height=48});
        if(visible.Count==0) panel.Children.Add(new TextBlock {Text="No Moments in this year.",Opacity=0.72});
        var cards=new StackPanel {Orientation=Orientation.Horizontal,Spacing=12};
        foreach(var moment in visible) {
            var id=moment["id"]?.GetValue<string>()??"";
            var title=moment["title"]?.GetValue<string>()??"Moment";
            var start=moment["start"]?.GetValue<long>()??0;
            var count=(moment["asset_ids"]?.AsArray()??new JsonArray()).Count;
            var featured=moment["featured"]?.GetValue<bool>()==true;
            var width=Math.Clamp(244+Math.Min(count,16)*7,252,360);
            var card=new StackPanel {Spacing=7,Width=width,Padding=new Thickness(14)};
            var top=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            var date=DateTimeOffset.FromUnixTimeSeconds(start).ToLocalTime().ToString("MMM d, yyyy");
            top.Children.Add(new TextBlock {Text=date,VerticalAlignment=VerticalAlignment.Center,Opacity=0.72});
            if(CanEditAssets(room)) {
                var feature=new Button {Content=featured?"★":"☆",Width=40,Height=36};
                Microsoft.UI.Xaml.Automation.AutomationProperties.SetName(feature,
                    featured?$"Remove {title} from Home spotlight":$"Feature {title} on Home");
                feature.Click+=async(_,_)=>{
                    if(core==null) return;
                    try {
                        var ids=(moment["asset_ids"]?.AsArray()??new JsonArray())
                            .Select(value=>value?.GetValue<string>()??"").Where(value=>value.Length>0).ToArray();
                        await core.Command(new {action="save_moment",room_id=roomId,id,title,
                            asset_ids=ids,featured=!featured});
                        await Reload();
                    } catch(Exception error) { Status.Text=$"Could not update Home spotlight: {error.Message}"; }
                };
                top.Children.Add(feature);
            } else if(featured) top.Children.Add(new TextBlock {Text="★",FontSize=20,Foreground=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(255,138,77,50))});
            card.Children.Add(top);
            var open=new Button {HorizontalAlignment=HorizontalAlignment.Stretch,HorizontalContentAlignment=HorizontalAlignment.Left,
                Padding=new Thickness(10),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(featured
                    ? Microsoft.UI.ColorHelper.FromArgb(255,246,231,219)
                    : Microsoft.UI.ColorHelper.FromArgb(255,248,246,242))};
            var content=new StackPanel {Spacing=8};
            var icon=new FontIcon {Glyph="",FontSize=26,Foreground=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(255,138,77,50))};
            content.Children.Add(new Border {Height=72,CornerRadius=new CornerRadius(10),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(255,233,228,222)),Child=icon});
            content.Children.Add(new TextBlock {Text=title,FontSize=19,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia"),TextTrimming=TextTrimming.CharacterEllipsis});
            content.Children.Add(new TextBlock {Text=$"{count} {(count==1?"memory":"memories")}",Opacity=0.72});
            open.Content=content;
            var momentCopy=moment.DeepClone();
            open.Click+=(_,_)=>{
                var choice=Navigation.MenuItems.OfType<NavigationViewItem>().FirstOrDefault(item=>item.Tag?.ToString()=="library");
                MomentFilter.SelectedValue=momentCopy["id"]?.GetValue<string>()??"";
                if(choice!=null) Navigation.SelectedItem=choice;
            };
            card.Children.Add(open);
            cards.Children.Add(new Border {Child=card,CornerRadius=new CornerRadius(16),BorderThickness=new Thickness(1),
                BorderBrush=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(45,70,60,50)),
                Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.ColorHelper.FromArgb(255,255,255,255))});
        }
        panel.Children.Add(new ScrollViewer {Content=cards,HorizontalScrollMode=ScrollMode.Enabled,
            HorizontalScrollBarVisibility=ScrollBarVisibility.Auto,VerticalScrollMode=ScrollMode.Disabled,
            VerticalScrollBarVisibility=ScrollBarVisibility.Disabled,Height=214});
        controls.Add(panel);
    }
    private static string MomentYear(long timestamp) {
        try { return DateTimeOffset.FromUnixTimeSeconds(timestamp).ToLocalTime().Year.ToString(); }
        catch(ArgumentOutOfRangeException) { return ""; }
    }
    private List<JsonNode> PlaylistMovies(JsonNode room,JsonNode playlist) {
        var movies=new List<JsonNode>();
        foreach(var id in playlist["story_ids"]?.AsArray()??new JsonArray()) {
            var story=room["stories"]?.AsArray().FirstOrDefault(candidate=>candidate?["id"]?.GetValue<string>()==id?.GetValue<string>() && candidate["private_draft"]?.GetValue<bool>()==false);
            var movieId=story?["published_asset"]?.GetValue<string>();
            var movie=room["assets"]?.AsArray().FirstOrDefault(candidate=>candidate?["id"]?.GetValue<string>()==movieId && candidate["deleted"]?.GetValue<bool>()!=true);
            if(movie==null) throw new InvalidOperationException("A film in this playlist is no longer available.");
            movies.Add(movie);
        }
        if(movies.Count==0) throw new InvalidOperationException("Add a film before playing this playlist.");
        return movies;
    }
    private async Task PlayPlaylist(JsonNode room,JsonNode playlist) {
        try {
            var movies=PlaylistMovies(room,playlist);
            screeningQueue.Clear(); screeningQueue.AddRange(movies);
            screeningTitle=playlist["title"]?.GetValue<string>()??"Playlist";
            screeningRoomId=room["id"]!.GetValue<string>();
            ScreeningPanel.Visibility=Visibility.Visible;
            ScreeningPanel.StartBringIntoView();
            await PlayFilm(0);
        } catch(Exception e) {Status.Text=e.Message;}
    }
    private static List<ScreeningCaptionCue> ReadScreeningCaptions(JsonNode? response)
    {
        if(response?["cues"] is not JsonArray encodedCues) return new();
        var result=new List<ScreeningCaptionCue>();
        foreach(var cue in encodedCues.Take(15_000))
        {
            if(!long.TryParse(cue?["start_ms"]?.ToString(),NumberStyles.None,CultureInfo.InvariantCulture,out var startMs) ||
               !long.TryParse(cue?["end_ms"]?.ToString(),NumberStyles.None,CultureInfo.InvariantCulture,out var endMs)) continue;
            var text=cue?["text"]?.GetValue<string>()?.Trim();
            if(endMs<=startMs || endMs-startMs>120_000 || string.IsNullOrWhiteSpace(text) || text.Length>1_024) continue;
            result.Add(new ScreeningCaptionCue(startMs,endMs,text));
        }
        return result.Distinct().OrderBy(cue=>cue.StartMs).ThenBy(cue=>cue.EndMs).ThenBy(cue=>cue.Text,StringComparer.Ordinal).ToList();
    }

    private void UpdateScreeningCaption()
    {
        var milliseconds=(long)(screeningMediaPlayer?.PlaybackSession.Position.TotalMilliseconds??0);
        var cue=SpokenCaptionsToggle.IsChecked==true
            ? screeningCaptions.FirstOrDefault(item=>item.IsActiveAt(milliseconds))
            : null;
        if(cue==null)
        {
            ScreeningCaptionPanel.Visibility=Visibility.Collapsed;
            return;
        }
        ScreeningCaptionText.Text=cue.Text;
        ScreeningCaptionPanel.Visibility=Visibility.Visible;
    }

    private void SpokenCaptionsChanged(object sender,RoutedEventArgs e)=>UpdateScreeningCaption();

    private void EnsureScreeningCaptionTimer()
    {
        if(screeningCaptionTimer!=null) return;
        screeningCaptionTimer=DispatcherQueue.CreateTimer();
        screeningCaptionTimer.Interval=TimeSpan.FromMilliseconds(200);
        screeningCaptionTimer.Tick+=(_,_)=>UpdateScreeningCaption();
        screeningCaptionTimer.Start();
    }

    private async Task PlayFilm(int index) {
        var seekAtMs=index==0?screeningStartMs:0; screeningStartMs=0;
        if(index<0 || index>=screeningQueue.Count) return;
        var generation=++screeningGeneration;
        screeningMediaPlayer?.Pause();
        screeningCaptions.Clear();
        SpokenCaptionsToggle.IsChecked=true;
        SpokenCaptionsToggle.IsEnabled=false;
        SpokenCaptionsToggle.Visibility=Visibility.Collapsed;
        ScreeningCaptionPanel.Visibility=Visibility.Collapsed;
        screeningIndex=index;
        ScreeningHeading.Text=$"{screeningTitle} · {index+1} of {screeningQueue.Count} · Loading…";
        PreviousFilm.IsEnabled=index>0; NextFilm.IsEnabled=index+1<screeningQueue.Count;
        try {
            var path=await MaterializeOriginal(screeningQueue[index],screeningRoomId);
            if(screeningGeneration!=generation) { File.Delete(path); return; }
            try
            {
                if(core!=null)
                {
                    var response=await core.Command(new {action="export_video_transcript",room_id=screeningRoomId,asset_id=screeningQueue[index]["id"]?.GetValue<string>()});
                    screeningCaptions.AddRange(ReadScreeningCaptions(response));
                }
            }
            catch { screeningCaptions.Clear(); }
            if(screeningGeneration!=generation) { File.Delete(path); return; }
            SpokenCaptionsToggle.IsEnabled=screeningCaptions.Count>0;
            SpokenCaptionsToggle.Visibility=screeningCaptions.Count>0?Visibility.Visible:Visibility.Collapsed;
            var file=await Windows.Storage.StorageFile.GetFileFromPathAsync(path);
            if(screeningGeneration!=generation) { File.Delete(path); return; }
            var player=new MediaPlayer();
            if(seekAtMs>0) player.MediaOpened+=(_,_)=>player.PlaybackSession.Position=TimeSpan.FromMilliseconds(seekAtMs);
            player.Source=MediaSource.CreateFromStorageFile(file);
            player.MediaEnded+=(sender,_)=>DispatcherQueue.TryEnqueue(async ()=>{
                if(screeningGeneration!=generation || !ReferenceEquals(screeningMediaPlayer,sender)) return;
                if(screeningIndex+1<screeningQueue.Count) await PlayFilm(screeningIndex+1);
                else ScreeningHeading.Text=$"{screeningTitle} · Playlist finished";
            });
            var previous=screeningMediaPlayer;
            screeningMediaPlayer=player;
            ScreeningPlayer.SetMediaPlayer(player);
            previous?.Dispose();
            EnsureScreeningCaptionTimer();
            ScreeningHeading.Text=$"{screeningTitle} · {index+1} of {screeningQueue.Count}";
            player.Play();
            UpdateScreeningCaption();
        } catch(Exception e) {Status.Text=e.Message;ScreeningHeading.Text=$"Film {index+1} is unavailable: {e.Message}";}
    }
    private async void PreviousFilmClicked(object sender,RoutedEventArgs args)=>await PlayFilm(screeningIndex-1);
    private async void NextFilmClicked(object sender,RoutedEventArgs args)=>await PlayFilm(screeningIndex+1);
    private void StopScreening() {
        screeningStartMs=0;
        screeningGeneration++;
        screeningMediaPlayer?.Pause();
        screeningMediaPlayer?.Dispose();
        screeningMediaPlayer=null;
        screeningCaptions.Clear();
        SpokenCaptionsToggle.Visibility=Visibility.Collapsed;
        ScreeningCaptionPanel.Visibility=Visibility.Collapsed;
        screeningQueue.Clear();
        ScreeningPanel.Visibility=Visibility.Collapsed;
    }
    private static string LabelFor(JsonNode? item) {
        var title=item?["title"]?.GetValue<string>();if(!string.IsNullOrWhiteSpace(title)) return title;
        var caption=item?["caption"]?.GetValue<string>();if(!string.IsNullOrWhiteSpace(caption)) return caption;
        return item?["filename"]?.GetValue<string>()??"Memory";
    }
    private async Task SaveCollection(bool playlist,string roomId,string? id,string title,List<string> ids,bool pinned=false) {
        title=title.Trim(); if(title.Length==0) { Status.Text="Give this collection a name."; return; }
        if(playlist) await Execute(new {action="save_playlist",room_id=roomId,id,title,story_ids=ids});
        else await Execute(new {action="save_album",room_id=roomId,id,title,asset_ids=ids,pinned});
    }
    private void RenderCollections(List<UIElement> controls,JsonNode room,bool playlist) {
        var roomId=room["id"]!.GetValue<string>();
        var editable=new[] {"owner","admin","contributor"}.Contains(room["role"]?.GetValue<string>());
        var collectionName=playlist ? "playlist" : "album";
        controls.Add(new TextBlock {Text=playlist ? "Playlists" : "Albums",FontSize=28,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia")});
        if(editable) {
            var create=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            var name=new TextBox {PlaceholderText=$"New {collectionName} name",Width=260};
            var button=new Button {Content=$"Create {collectionName}"};
            button.Click+=async(_,_)=>await SaveCollection(playlist,roomId,null,name.Text,new List<string>());
            create.Children.Add(name);create.Children.Add(button);controls.Add(create);
        }
        var candidates=(playlist ? room["stories"] : room["assets"])?.AsArray()
            .Where(item=>item!=null && (playlist
                ? item["private_draft"]?.GetValue<bool>()==false && item["published_asset"]!=null
                : item["deleted"]?.GetValue<bool>()!=true))
            .Select(item=>item!).ToList() ?? new List<JsonNode>();
        var entries=(playlist ? room["playlists"] : room["albums"])?.AsArray();
        if(entries==null) return;
        foreach(var item in entries) {
            if(item==null) continue;
            var id=item["id"]!.GetValue<string>();var title=item["title"]?.GetValue<string>()??"Untitled";
            var pinned=item["pinned"]?.GetValue<bool>()==true;
            var key=playlist ? "story_ids" : "asset_ids";
            var order=item[key]?.AsArray().Select(value=>value!.GetValue<string>()).ToList() ?? new List<string>();
            var panel=new StackPanel {Spacing=8,Margin=new Thickness(0,8,0,18)};
            var unit=playlist ? (order.Count==1?"film":"films") : (order.Count==1?"memory":"memories");
            panel.Children.Add(new TextBlock {Text=$"{title} · {order.Count} {unit}",FontSize=20,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia")});
            if(!playlist) {
                var caption=item["caption"]?.GetValue<string>()??"";
                var coverId=item["cover_asset_id"]?.GetValue<string>();
            var coverMemory=candidates.FirstOrDefault(candidate=>candidate["id"]?.GetValue<string>()==(coverId??order.FirstOrDefault()));
            if(coverMemory!=null) panel.Children.Add(new TextBlock {Text=$"Cover · {LabelFor(coverMemory)}",Opacity=0.72});
            if(!string.IsNullOrWhiteSpace(caption)) panel.Children.Add(new TextBlock {Text=caption,TextWrapping=TextWrapping.Wrap,Opacity=0.8});
            if(!playlist) { var allRoomAssets=room["assets"]?.AsArray().Where(asset=>asset!=null).Select(asset=>asset!).ToList()??new List<JsonNode>(); var albumAssetIds=new HashSet<string>(order); foreach(var parent in allRoomAssets.Where(asset=>order.Contains(asset["id"]?.GetValue<string>()??""))) foreach(var component in parent["components"]?.AsArray()??new JsonArray()) if(component!=null) albumAssetIds.Add(component.GetValue<string>()); var offlineAssets=allRoomAssets.Where(asset=>albumAssetIds.Contains(asset["id"]?.GetValue<string>()??"") && asset["deleted"]?.GetValue<bool>()!=true).ToList(); var offlineCount=offlineAssets.Count(asset=>asset["pinned_offline"]?.GetValue<bool>()==true); var allOffline=offlineAssets.Count>0 && offlineCount==offlineAssets.Count; var offlineButton=new Button {Content=allOffline?"Unpin originals":$"Save originals offline ({offlineCount}/{offlineAssets.Count})",IsEnabled=offlineAssets.Count>0}; offlineButton.Click+=async(_,_)=>{ offlineButton.IsEnabled=false; try { var action=allOffline?"unpin_original":"pin_original"; foreach(var asset in offlineAssets) { var isPinned=asset["pinned_offline"]?.GetValue<bool>()==true; if((allOffline&&!isPinned)||(!allOffline&&isPinned)) continue; await core!.Command(new {action,room_id=roomId,asset_id=asset["id"]!.GetValue<string>()}); } await Open(); } catch(Exception error) { try { await Open(); } catch { } Status.Text=error.Message; offlineButton.IsEnabled=true; } }; panel.Children.Add(offlineButton); var evictableAssets=offlineAssets.Where(asset=>asset["local"]?.GetValue<bool>()==true && asset["pinned_offline"]?.GetValue<bool>()!=true && (asset["verified_copies"]?.GetValue<int>()??0)>1).ToList(); if(evictableAssets.Count>0) { var freeButton=new Button {Content=$"Free device storage ({evictableAssets.Count})",HorizontalAlignment=HorizontalAlignment.Left}; freeButton.Click+=async(_,_)=>{ var confirm=new ContentDialog {Title="Free device storage?",Content=new TextBlock {Text="Only unpinned originals with a second verified copy can be removed. They can be downloaded again later.",TextWrapping=TextWrapping.Wrap},PrimaryButtonText="Free space",CloseButtonText="Cancel",DefaultButton=ContentDialogButton.Primary,XamlRoot=ContentItems.XamlRoot}; if(await confirm.ShowAsync()!=ContentDialogResult.Primary) return; freeButton.IsEnabled=false; try { foreach(var asset in evictableAssets) await core!.Command(new {action="evict_original",room_id=roomId,asset_id=asset["id"]!.GetValue<string>()}); await Open(); } catch(Exception error) { try { await Open(); } catch { } Status.Text=error.Message; freeButton.IsEnabled=true; } }; panel.Children.Add(freeButton); } } if(CanPublishAlbum(room)) {
                var share=new Button {Content="Share privately",HorizontalAlignment=HorizontalAlignment.Left};
                share.Click+=async(_,_)=>await ShareAlbum(room,item);
                panel.Children.Add(share);
            }
            if(editable) {
                    var captionRow=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
                    var captionBox=new TextBox {Text=caption,PlaceholderText="A note about this album",Width=340,MaxLength=2000};
                    var saveCaption=new Button {Content="Save note"};
                    saveCaption.Click+=async(_,_)=>await Execute(new {action="save_album",room_id=roomId,id,title,asset_ids=order,pinned,caption=captionBox.Text});
                    captionRow.Children.Add(captionBox);captionRow.Children.Add(saveCaption);panel.Children.Add(captionRow);
                }
            }
            if(playlist) {
                var playAll=new Button {Content="Play all",IsEnabled=order.Count>0};
                playAll.Click+=async(_,_)=>await PlayPlaylist(room,item);
                panel.Children.Add(playAll);
            }
            if(editable && playlist) {
                var delete=new Button {Content="Delete playlist"};
                delete.Click+=async(_,_)=>{
                    var confirmation=new ContentDialog {XamlRoot=Content.XamlRoot,Title="Delete this playlist?",
                        Content="The films and their originals stay in the Room.",PrimaryButtonText="Delete playlist",CloseButtonText="Cancel"};
                    if(await confirmation.ShowAsync()==ContentDialogResult.Primary)
                        await Execute(new {action="delete_playlist",room_id=roomId,id});
                };
                panel.Children.Add(delete);
            }
            if(editable) {
                var rename=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
                var name=new TextBox {Text=title,Width=260};var save=new Button {Content="Save name"};
                save.Click+=async(_,_)=>await SaveCollection(playlist,roomId,id,name.Text,order,pinned);
                rename.Children.Add(name);rename.Children.Add(save);panel.Children.Add(rename);
                var available=candidates.Where(candidate=>!order.Contains(candidate["id"]?.GetValue<string>()??"")).ToList();
                if(available.Count>0) {
                    var addRow=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
                    var choice=new ComboBox {Width=260,ItemsSource=available.Select(candidate=>new CollectionChoice(candidate["id"]!.GetValue<string>(),LabelFor(candidate))).ToList(),DisplayMemberPath="Name",SelectedIndex=0};
                    var add=new Button {Content=playlist?"Add film":"Add memory"};
                    add.Click+=async(_,_)=>{ if(choice.SelectedItem is CollectionChoice selected) await SaveCollection(playlist,roomId,id,title,order.Append(selected.Id).ToList(),pinned); };
                    addRow.Children.Add(choice);addRow.Children.Add(add);panel.Children.Add(addRow);
                }
            }
            for(var index=0;index<order.Count;index++) {
                var current=index;var assetId=order[index];
                var candidate=candidates.FirstOrDefault(candidate=>candidate["id"]?.GetValue<string>()==assetId);
                var line=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
                line.Children.Add(new TextBlock {Text=$"{index+1}. {LabelFor(candidate)}",Width=260});
                if(playlist && candidate?["published_asset"]?.GetValue<string>() is string movieId) {
                    var movie=room["assets"]?.AsArray().FirstOrDefault(asset=>asset?["id"]?.GetValue<string>()==movieId);
                    if(movie!=null && movie["deleted"]?.GetValue<bool>()!=true) {
                        var watch=new Button {Content="Watch"};
                        watch.Click+=async(_,_)=>await OpenOriginal(movie,roomId);
                        line.Children.Add(watch);
                    }
                }
                if(editable) {
                    if(!playlist) {
                        var cover=new Button {Content="Make cover"};
                        cover.Click+=async(_,_)=>await Execute(new {action="save_album",room_id=roomId,id,title,asset_ids=order,pinned,cover_asset_id=assetId});
                        line.Children.Add(cover);
                    }
                    var up=new Button {Content="Move earlier",IsEnabled=current>0};
                    up.Click+=async(_,_)=>{var next=order.ToList();(next[current-1],next[current])=(next[current],next[current-1]);await SaveCollection(playlist,roomId,id,title,next,pinned);};
                    var down=new Button {Content="Move later",IsEnabled=current+1<order.Count};
                    down.Click+=async(_,_)=>{var next=order.ToList();(next[current+1],next[current])=(next[current],next[current+1]);await SaveCollection(playlist,roomId,id,title,next,pinned);};
                    var remove=new Button {Content="Remove"};
                    remove.Click+=async(_,_)=>await SaveCollection(playlist,roomId,id,title,order.Where((_,position)=>position!=current).ToList(),pinned);
                    line.Children.Add(up);line.Children.Add(down);line.Children.Add(remove);
                }
                panel.Children.Add(line);
            }
            controls.Add(panel);
        }
    }
    private static bool CanPublishAlbum(JsonNode room) =>
        room["album_only"]?.GetValue<bool>()!=true && new[]{"owner","admin"}.Contains(room["role"]?.GetValue<string>());
    private static List<string> AlbumPublicationAssetIds(JsonNode room,JsonNode album) {
        var assets=room["assets"]?.AsArray()??new JsonArray();
        var selected=new List<string>();
        foreach(var value in album["asset_ids"]?.AsArray()??new JsonArray()) {
            var id=value?.GetValue<string>();
            if(string.IsNullOrWhiteSpace(id)) continue;
            var asset=assets.FirstOrDefault(candidate=>candidate?["id"]?.GetValue<string>()==id && candidate["deleted"]?.GetValue<bool>()!=true);
            if(asset==null) continue;
            if(!selected.Contains(id)) selected.Add(id);
            foreach(var componentValue in asset["components"]?.AsArray()??new JsonArray()) {
                var component=componentValue?.GetValue<string>();
                if(string.IsNullOrWhiteSpace(component)) continue;
                if(assets.Any(candidate=>candidate?["id"]?.GetValue<string>()==component && candidate["deleted"]?.GetValue<bool>()!=true) && !selected.Contains(component)) selected.Add(component);
            }
        }
        return selected;
    }
    private async Task ShareAlbum(JsonNode room,JsonNode album) {
        if(core==null || !CanPublishAlbum(room)) return;
        var roomId=room["id"]!.GetValue<string>();
        var albumId=album["id"]!.GetValue<string>();
        var albumTitle=album["title"]?.GetValue<string>()??"this album";
        var device=new TextBox {PlaceholderText="Viewer’s device code",MinWidth=300};
        var invitation=new TextBox {IsReadOnly=true,AcceptsReturn=true,TextWrapping=TextWrapping.Wrap,MaxHeight=150,Visibility=Visibility.Collapsed};
        var status=new TextBlock {TextWrapping=TextWrapping.Wrap,Opacity=0.78};
        var publish=new Button {Content="Publish/update current viewers"};
        var createInvite=new Button {Content="Create album-only invitation"};
        var content=new StackPanel {Spacing=10};
        content.Children.Add(new TextBlock {Text="Viewers receive a separately encrypted copy of this album, without access to the rest of the Room. Person names do not grant access. Copies already downloaded cannot be recalled.",TextWrapping=TextWrapping.Wrap});
        content.Children.Add(device);
        content.Children.Add(invitation);
        content.Children.Add(status);
        var dialog=new ContentDialog {Title=$"Share only “{albumTitle}”",Content=content,CloseButtonText="Done",DefaultButton=ContentDialogButton.None,XamlRoot=MembersPanel.XamlRoot};
        async Task PrepareAndRun(bool invite) {
            var ids=AlbumPublicationAssetIds(room,album);
            if(ids.Count==0) { status.Text="Add an available memory to this album before sharing."; return; }
            if(invite && string.IsNullOrWhiteSpace(device.Text)) { status.Text="Enter the intended viewer’s device code."; return; }
            publish.IsEnabled=false;
            createInvite.IsEnabled=false;
            status.Text="Preparing album originals…";
            try {
                foreach(var id in ids) {
                    var asset=room["assets"]?.AsArray().FirstOrDefault(candidate=>candidate?["id"]?.GetValue<string>()==id);
                    if(asset?["local"]?.GetValue<bool>()!=true) {
                        await UpdateNetworkState();
                        await core.Command(new {action="fetch",room_id=roomId,asset_id=id});
                    }
                }
                var result=invite
                    ? await core.Command(new {action="invite_album",room_id=roomId,album_id=albumId,device_id=device.Text.Trim()})
                    : await core.Command(new {action="publish_album",room_id=roomId,album_id=albumId});
                await Sync();
                if(invite) {
                    invitation.Text=result["invitation"]?.GetValue<string>()??"";
                    invitation.Visibility=Visibility.Visible;
                    status.Text="Invitation created. Copy the selected token and send it only to the intended viewer.";
                    device.Text="";
                } else {
                    status.Text=$"Published or updated {result["count"]?.GetValue<int>()??0} album memories for current viewers.";
                }
            } catch(Exception e) { status.Text=e.Message; }
            finally { publish.IsEnabled=true; createInvite.IsEnabled=true; }
        }
        publish.Click+=async(_,_)=>await PrepareAndRun(false);
        createInvite.Click+=async(_,_)=>await PrepareAndRun(true);
        content.Children.Add(publish);
        content.Children.Add(createInvite);
        await dialog.ShowAsync();
    }
    private async void CreateRoom(object sender,RoutedEventArgs args) => await Execute(new {action="create_room",name=RoomName.Text});
    private void InvitationChanged(object sender,TextChangedEventArgs args) {
        if(reviewedInvitation==null||String.Equals(reviewedInvitation,Invitation.Text.Trim(),StringComparison.Ordinal)) return;
        reviewedInvitation=null;
        InvitationPreviewPanel.Visibility=Visibility.Collapsed;
    }
    private async void ReviewInvitation(object sender,RoutedEventArgs args) {
        if(core==null||string.IsNullOrWhiteSpace(Invitation.Text)) return;
        try {
            var token=Invitation.Text.Trim();
            var preview=await core.Command(new {action="preview_invitation",invitation=token});
            reviewedInvitation=token;
            var name=preview["name"]?.GetValue<string>()??"Room";
            var owner=preview["owner_id"]?.GetValue<string>()??"";
            var role=preview["role"]?.GetValue<string>()??"viewer";
            var albumOnly=preview["album_only"]?.GetValue<bool>()==true;
            InvitationPreviewText.Text=$"You’re invited to {name}\nOwner device {owner[..Math.Min(8,owner.Length)]} · {role} access"+(albumOnly?"\nThis invitation grants access only to its shared album.":"");
            InvitationPreviewPanel.Visibility=Visibility.Visible;
            Status.Text="Review the Room and access level before saving or accepting.";
        } catch(Exception e) {
            reviewedInvitation=null;
            InvitationPreviewPanel.Visibility=Visibility.Collapsed;
            Status.Text=e.Message;
        }
    }
    private async void StageInvitation(object sender,RoutedEventArgs args) {
        if(core==null||reviewedInvitation==null) return;
        try {
            await core.Command(new {action="stage_invitation",invitation=reviewedInvitation});
            reviewedInvitation=null;
            Invitation.Text="";
            InvitationPreviewPanel.Visibility=Visibility.Collapsed;
            await Reload();
            Status.Text="Invitation saved for review on this device.";
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private void RenderPendingInvitations() {
        if(PendingInvitations==null) return;
        PendingInvitations.Children.Clear();
        var rows=snapshot?["pending_invitations"]?.AsArray();
        if(rows==null||rows.Count==0) { PendingInvitations.Visibility=Visibility.Collapsed; return; }
        PendingInvitations.Visibility=Visibility.Visible;
        foreach(var row in rows) {
            if(row==null) continue;
            var id=row["id"]?.GetValue<string>()??"";
            var room=row["room_id"]?.GetValue<string>()??"";
            var owner=row["owner_id"]?.GetValue<string>()??"";
            var role=row["role"]?.GetValue<string>()??"viewer";
            var card=new StackPanel {Spacing=6,Padding=new Thickness(12)};
            card.Children.Add(new TextBlock {Text=row["name"]?.GetValue<string>()??"Room invitation",FontSize=17,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold});
            card.Children.Add(new TextBlock {Text=$"Device {owner[..Math.Min(8,owner.Length)]} · {role} access"+(row["album_only"]?.GetValue<bool>()==true?" · shared album only":""),TextWrapping=TextWrapping.Wrap,Opacity=0.75});
            var actions=new StackPanel {Orientation=Orientation.Horizontal,Spacing=8};
            var accept=new Button {Content="Accept"};
            accept.Click+=async(_,_)=>{try{if(core==null)return;var result=await core.Command(new {action="accept_invitation",id});await Reload();Rooms.SelectedValue=result["id"]?.GetValue<string>()??room;await Reload();Status.Text="Joined the Room.";}catch(Exception e){Status.Text=e.Message;}};
            var decline=new Button {Content="Decline"};
            decline.Click+=async(_,_)=>{try{if(core==null)return;await core.Command(new {action="decline_invitation",id});await Reload();Status.Text="Invitation removed from this device. The sender is not notified.";}catch(Exception e){Status.Text=e.Message;}};
            actions.Children.Add(accept);actions.Children.Add(decline);card.Children.Add(actions);PendingInvitations.Children.Add(card);
        }
    }
    private async void ConnectGateway(object sender,RoutedEventArgs args) {
        if(Room==null) return; await Execute(new {action="add_source",room_id=Room["id"]!.GetValue<string>(),name="Room gateway",kind="gateway",endpoint=Gateway.Text}); await Sync();
    }
    private async Task Sync() {
        if(core==null||Room==null) return;
        try { await UpdateNetworkState(); var result=await core.Command(new {action="sync_all",room_id=Room["id"]!.GetValue<string>()}); await Reload(); var errors=result["errors"]!.AsArray(); var cleanup=result["cache_cleanup"]; Status.Text=errors.Count>0 ? errors.ToJsonString() : cleanup?["enabled"]?.GetValue<bool>()==true && cleanup["complete"]?.GetValue<bool>()==false ? "Room synchronized. Automatic cache cleanup is waiting for another verified copy; originals remain saved here." : "Room synchronized."; } catch(Exception e) {Status.Text=e.Message;}
    }
    private async void SyncRoom(object sender,RoutedEventArgs args) => await Sync();
    private async void InviteMember(object sender,RoutedEventArgs args) {
        if(core==null || Room==null) return;
        var roomId=Room["id"]!.GetValue<string>();
        try {
            var result=await core.Command(new {action="invite",room_id=roomId,device_id=InviteDevice.Text,
                role=(InviteRole.SelectedItem as ComboBoxItem)?.Content?.ToString()??"contributor"});
            GeneratedInvitation.Text=result["invitation"]!.GetValue<string>();
            var published=false;
            foreach(var source in snapshot?["sources"]?.AsArray().Where(source=>
                source?["room_id"]?.GetValue<string>()==roomId && source["paused"]?.GetValue<bool>()!=true)
                .ToList()??new List<JsonNode?>()) {
                try {
                    await core.Command(new {action="refresh_access",id=source!["id"]!.GetValue<string>()});
                    published=true;
                } catch { }
            }
            await Reload();
            Status.Text=published ? "Invitation ready. Connected storage is prepared for this Member."
                : "Invitation ready. Sync the Room before the new Member connects.";
        } catch(Exception e) { Status.Text=e.Message; }
    }
    private async void ImportMedia(object sender,RoutedEventArgs args) {
        if(Room==null) return; var roomId=Room["id"]!.GetValue<string>(); var picker=new FileOpenPicker(); WinRT.Interop.InitializeWithWindow.Initialize(picker,WinRT.Interop.WindowNative.GetWindowHandle(this)); picker.FileTypeFilter.Add("*"); var files=await picker.PickMultipleFilesAsync(); foreach(var file in files) await Execute(new {action="import",room_id=roomId,path=file.Path});
    }
    private void SectionChanged(NavigationView sender,NavigationViewSelectionChangedEventArgs args) {
        if(args.SelectedItem is not NavigationViewItem item) return;
        var nextSection=item.Tag?.ToString()??"home";
        if(nextSection!=section) { selectingAssets=false; selectedAssets.Clear(); }
        if(nextSection!="settings") { RecoveryKeyText.Text=""; RecoveryKeyText.Visibility=Visibility.Collapsed; RevealRecoveryKeyButton.Content="Show recovery key"; RecoveryKeyInput.Password=""; }
        section=nextSection;
        if(section!="theater"&&section!="video-finder") StopScreening();
        Render();
    }
    private void RoomChanged(object sender,SelectionChangedEventArgs args) { StopScreening(); selectingAssets=false; selectedAssets.Clear(); lastVideoPrompt=null; Render(); }
    private void SearchChanged(object sender,TextChangedEventArgs args)=>Render();
    private void VideoQuestionKeyDown(object sender,Microsoft.UI.Xaml.Input.KeyRoutedEventArgs args) {
        if(args.Key==Windows.System.VirtualKey.Enter) { args.Handled=true; _=SearchVideoQuestion(); }
    }
    private void ClearVideoSearch(object sender,RoutedEventArgs args) { lastVideoPrompt=null;
        VideoQuestion.Text=""; VideoCoverage.Text=""; VideoResults.ItemsSource=Array.Empty<UIElement>(); Status.Text="";
    }
    private async void SearchVideos(object sender,RoutedEventArgs args) => await SearchVideoQuestion();
    private async Task SearchVideoQuestion() {
        var room=Room; if(core==null||room==null||VideoQuestion.Text.Trim().Length==0) return;
        AskVideos.IsEnabled=false; IncludeRemoteVideos.IsEnabled=false; var includeRemote=IncludeRemoteVideos.IsChecked==true; VideoCoverage.Text="Checking videos in this Room…";
        var transcript=(VideoResults.ItemsSource as IEnumerable<UIElement>)?.ToList()??new List<UIElement>();
        transcript.Add(new Border {Child=new TextBlock {Text=VideoQuestion.Text.Trim(),TextWrapping=TextWrapping.Wrap,HorizontalAlignment=HorizontalAlignment.Right},Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Transparent),Padding=new Thickness(14),HorizontalAlignment=HorizontalAlignment.Right,Margin=new Thickness(0,10,0,4)});
        VideoResults.ItemsSource=transcript.ToList();
        try {
            var roomId=room["id"]!.GetValue<string>(); var prompt=VideoQuestion.Text.Trim();
            var selected=VideoModality.SelectedItem as ComboBoxItem; var modality=selected?.Tag?.ToString()??"all";
            var status=await core.Command(new {action="video_index_status",room_id=roomId});
            var visionUrl=Environment.GetEnvironmentVariable("FAMILY_ROOM_VISION_URL")??"";
            var visionConfigured=visionUrl.StartsWith("http://127.0.0.1:",StringComparison.OrdinalIgnoreCase)||
                visionUrl.StartsWith("http://localhost:",StringComparison.OrdinalIgnoreCase)||
                visionUrl.StartsWith("http://[::1]:",StringComparison.OrdinalIgnoreCase);
            var unindexedVideos=status["videos"]!.AsArray().Where(video=>{
                var framesMissing=video?["frames_indexed"]?.GetValue<bool>()!=true;
                var audioMissing=video?["audio_indexed"]?.GetValue<bool>()!=true;
                var visualMissing=modality!="audio"&&visionConfigured&&video?["visual_labels_indexed"]?.GetValue<bool>()!=true;
                var requestedEvidenceMissing=(modality switch {"frame"=>framesMissing,"audio"=>audioMissing,_=>framesMissing||audioMissing})||visualMissing;
                return (includeRemote||video?["local"]?.GetValue<bool>()==true)&&requestedEvidenceMissing;
            }).ToList();
            var videoCount=status["videos"]!.AsArray().Count;
            var indexingWarning="";
            for(var index=0;index<unindexedVideos.Count;index++) {
                var id=unindexedVideos[index]!["asset_id"]!.GetValue<string>();
                var asset=room["assets"]!.AsArray().FirstOrDefault(candidate=>candidate?["id"]?.GetValue<string>()==id&&candidate["deleted"]?.GetValue<bool>()!=true);
                if(asset==null) continue;
                var alreadyLocal=unindexedVideos[index]?["local"]?.GetValue<bool>()==true;
                var indexingAction=alreadyLocal?"Indexing local":"Fetching and indexing remote";
                VideoCoverage.Text=$"{indexingAction} video {index+1} of {unindexedVideos.Count}…";
                var path="";
                try {
                    path=await MaterializeVideo(asset,roomId,alreadyLocal);
                    var analysis=await RunVideoIndexer(path,modality);
                    await core.Command(new {action="index_video",room_id=roomId,asset_id=id,
                        observations=analysis["observations"]?.DeepClone(),
                        audio_indexed=analysis["audio_indexed"]?.GetValue<bool>()??false,
                        frames_indexed=analysis["frames_indexed"]?.GetValue<bool>()??false,
                        visual_labels_indexed=analysis["visual_labels_indexed"]?.GetValue<bool>()??false,
                        frame_interval_ms=analysis["frame_interval_ms"]?.GetValue<ulong>()??0,
                        preserve_existing=true});
                    var visualMessage=analysis["visual_message"]?.GetValue<string>();
                    if(!string.IsNullOrWhiteSpace(visualMessage))
                        indexingWarning+=(indexingWarning.Length>0?" ":"")+visualMessage;
                    if(modality!="frame"&&analysis["audio_indexed"]?.GetValue<bool>()!=true)
                        indexingWarning=analysis["audio_message"]?.GetValue<string>()??"Speech analysis is unavailable on this device.";
                    if(modality!="audio"&&analysis["frames_indexed"]?.GetValue<bool>()!=true)
                        indexingWarning+=(indexingWarning.Length>0?" ":"")+"Frame text is unavailable; install FFmpeg and Tesseract.";
                } catch(Exception error) { indexingWarning=error.Message; break; }
                finally { if(path.Length>0) try { File.Delete(path); Directory.Delete(Path.GetDirectoryName(path)!,true); } catch {} }
            }
            var result=await core.Command(new {action="search_video",room_id=roomId,query=prompt,modality,context_query=lastVideoPrompt}); lastVideoPrompt=prompt;
            var coverage=result["coverage"]!;
            VideoCoverage.Text=$"{coverage["indexed"]} of {coverage["videos"]} Room videos indexed here · {coverage["audio_indexed"]} with speech · {coverage["frames_indexed"]} with frames"+(videoCount>0?"":" · No videos in this Room yet.");
            var controls=transcript;
            var answer=result["answer"]?.GetValue<string>()??"No matching moment was found in indexed videos.";
            controls.Add(new TextBlock {Text=answer,FontSize=20,FontFamily=new Microsoft.UI.Xaml.Media.FontFamily("Georgia"),TextWrapping=TextWrapping.Wrap,Margin=new Thickness(0,8,0,8)});
            foreach(var match in result["matches"]?.AsArray()??new JsonArray()) {
                if(match==null) continue; var assetId=match["asset_id"]!.GetValue<string>();
                var matchedAsset=room["assets"]!.AsArray().FirstOrDefault(candidate=>candidate?["id"]?.GetValue<string>()==assetId&&candidate["deleted"]?.GetValue<bool>()!=true);
                if(matchedAsset==null) continue;
                var card=new StackPanel {Spacing=7,Margin=new Thickness(0,10,0,10),Padding=new Thickness(16),Background=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.Transparent)};
                card.Children.Add(new TextBlock {Text=LabelFor(matchedAsset),FontSize=18,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold,TextWrapping=TextWrapping.Wrap});
            var startMs=match["start_ms"]?.GetValue<ulong>()??0;
            var endMs=match["end_ms"]?.GetValue<ulong>()??startMs;
            var contextEvidence=match["context_evidence"]?.AsArray()??new JsonArray();
            var matchKind=match["match_kind"]?.GetValue<string>()??"suggestion"; var isExact=matchKind=="exact"; var matchLabel=matchKind switch {"exact"=>"EXACT PHRASE","evidence"=>"INDEXED EVIDENCE",_=>"SUGGESTED MATCH"};
            if(isExact&&contextEvidence.Count>0) matchLabel+=" · NEARBY CLUE";
            card.Children.Add(new TextBlock {Text=matchLabel,FontSize=12,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold,Opacity=matchKind=="suggestion"?0.72:1});
            var timeRange=endMs>startMs?$"{VideoTime(startMs)}–{VideoTime(endMs)}":$"At {VideoTime(startMs)}";
                var open=new Button {Content=$"Open {timeRange}"}; var capturedAsset=matchedAsset.DeepClone();
                open.Click+=(_,_)=>OpenVideoMatch(capturedAsset,roomId,startMs); card.Children.Add(open);
                if(contextEvidence.Count>0) {
                    card.Children.Add(new TextBlock {Text="NEARBY CLUE",FontSize=11,FontWeight=Microsoft.UI.Text.FontWeights.SemiBold,Opacity=0.78});
                    foreach(var clue in contextEvidence.Take(2)) {
                        if(clue==null) continue;
                        var kind=clue["modality"]?.GetValue<string>() switch {"speech"=>"Spoken","screen_text"=>"On screen","sound_event"=>"Possible sound",_=>"Visual suggestion"};
                        var text=clue["text"]?.GetValue<string>()??"";
                        card.Children.Add(new TextBlock {Text=$"{kind} · {VideoTime(clue["start_ms"]?.GetValue<ulong>()??startMs)}\n{text}",TextWrapping=TextWrapping.Wrap,Opacity=0.78});
                    }
                }
                foreach(var evidence in match["evidence"]?.AsArray()??new JsonArray()) {
                    if(evidence==null) continue;
                    var kind=evidence["modality"]?.GetValue<string>() switch {"speech"=>"Spoken","screen_text"=>"On screen",_=>"Visual suggestion"};
                    var text=evidence["text"]?.GetValue<string>()??"";
                    card.Children.Add(new TextBlock {Text=$"{kind} · {VideoTime(evidence["start_ms"]?.GetValue<ulong>()??startMs)}\n{text}",TextWrapping=TextWrapping.Wrap,Opacity=0.78});
                }
                controls.Add(new Border {Child=card,BorderBrush=new Microsoft.UI.Xaml.Media.SolidColorBrush(Microsoft.UI.Colors.LightGray),BorderThickness=new Thickness(1),CornerRadius=new CornerRadius(16),Margin=new Thickness(0,4,0,4)});
            }
            VideoResults.ItemsSource=controls;
            Status.Text=indexingWarning.Length>0?$"{indexingWarning} Search also checked previously indexed device evidence.":includeRemote?"Search ran on this device and included selected remote-only videos.":"Search ran on this device. Remote-only videos were skipped.";
        } catch(Exception error) { VideoCoverage.Text=""; Status.Text=error.Message; transcript.Add(new TextBlock {Text=error.Message,TextWrapping=TextWrapping.Wrap,Opacity=0.75,Margin=new Thickness(8)}); VideoResults.ItemsSource=transcript.ToList(); }
        finally { AskVideos.IsEnabled=true; IncludeRemoteVideos.IsEnabled=true; }
    }
    private async Task<string> MaterializeVideo(JsonNode asset,string roomId,bool alreadyLocal) {
        if(!alreadyLocal) await core!.Command(new {action="fetch",room_id=roomId,asset_id=asset["id"]!.GetValue<string>()});
        var root=Path.Combine(Path.GetTempPath(),"FamilyRoom","video-index",Guid.NewGuid().ToString("N")); Directory.CreateDirectory(root);
        var path=Path.Combine(root,Path.GetFileName(asset["filename"]?.GetValue<string>()??"video.mp4"));
        await core!.Command(new {action="materialize",room_id=roomId,asset_id=asset["id"]!.GetValue<string>(),destination=path}); return path;
    }
    private static async Task<JsonNode> RunVideoIndexer(string path,string modality) {
        var executable=Path.Combine(AppContext.BaseDirectory,"family-video-indexer.exe");
        if(!File.Exists(executable)) throw new InvalidOperationException("The local video-indexing helper is missing from this installation.");
        var start=new ProcessStartInfo(executable) {UseShellExecute=false,RedirectStandardOutput=true,RedirectStandardError=true,CreateNoWindow=true};
        start.ArgumentList.Add("--input"); start.ArgumentList.Add(path);
        if(modality=="frame") start.ArgumentList.Add("--frame-only");
        else if(modality=="audio") start.ArgumentList.Add("--audio-only");
        using var process=Process.Start(start)??throw new InvalidOperationException("Could not start local video indexing.");
        var outputTask=process.StandardOutput.ReadToEndAsync(); var errorTask=process.StandardError.ReadToEndAsync(); await process.WaitForExitAsync();
        var output=await outputTask; var error=await errorTask;
        if(process.ExitCode!=0) throw new InvalidOperationException(string.IsNullOrWhiteSpace(error)?"Local video indexing failed.":error.Trim());
        return JsonNode.Parse(output)??throw new InvalidOperationException("Local video indexing returned invalid evidence.");
    }
    private void OpenVideoMatch(JsonNode asset,string roomId,ulong startMs) {
        screeningQueue.Clear(); screeningQueue.Add(asset); screeningRoomId=roomId; screeningTitle="Video search result"; screeningStartMs=startMs; ScreeningPanel.Visibility=Visibility.Visible; ScreeningPanel.StartBringIntoView(); PlayFilm(0);
    }
    private static string VideoTime(ulong milliseconds)=>$"{milliseconds/60000:00}:{milliseconds/1000%60:00}";
    private sealed record RoomChoice(string Id,string Name);
    private sealed record CollectionChoice(string Id,string Name);
}
