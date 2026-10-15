using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;
namespace FamilyRoom;
internal sealed class NativeCore : IDisposable {
    [DllImport("family_core", CallingConvention=CallingConvention.Cdecl)] private static extern ulong fr_request_new();
    [DllImport("family_core", CallingConvention=CallingConvention.Cdecl)] private static extern byte fr_request_push(ulong handle, ulong word, byte length);
    [DllImport("family_core", CallingConvention=CallingConvention.Cdecl)] private static extern ulong fr_dispatch(ulong core, ulong request);
    [DllImport("family_core", CallingConvention=CallingConvention.Cdecl)] private static extern ulong fr_response_length(ulong handle);
    [DllImport("family_core", CallingConvention=CallingConvention.Cdecl)] private static extern ulong fr_response_word(ulong handle, ulong index);
    [DllImport("family_core", CallingConvention=CallingConvention.Cdecl)] private static extern void fr_release(ulong handle);
    private ulong handle;
    private readonly SemaphoreSlim queue = new(1, 1);
    private readonly string recoveryKeyPath;
    public NativeCore() {
        var root = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "FamilyRoom"); Directory.CreateDirectory(root);
        recoveryKeyPath = Path.Combine(root, "key.dpapi"); byte[] key;
        if (File.Exists(recoveryKeyPath)) key = ProtectedData.Unprotect(File.ReadAllBytes(recoveryKeyPath), null, DataProtectionScope.CurrentUser);
        else { if (File.Exists(Path.Combine(root, "Vault", "catalog.fr"))) throw new InvalidOperationException("Recover the existing vault key before opening this library."); key = RandomNumberGenerator.GetBytes(32); File.WriteAllBytes(recoveryKeyPath, ProtectedData.Protect(key, null, DataProtectionScope.CurrentUser)); }
        try { handle = Call(0, new { directory=Path.Combine(root,"Vault"), recovery_key=Convert.ToHexString(key).ToLowerInvariant() })["handle"]!.GetValue<ulong>(); }
        finally { CryptographicOperations.ZeroMemory(key); }
    }
    public string RevealRecoveryKey() {
        var key = ProtectedData.Unprotect(File.ReadAllBytes(recoveryKeyPath), null, DataProtectionScope.CurrentUser);
        try { return Convert.ToHexString(key).ToLowerInvariant(); }
        finally { CryptographicOperations.ZeroMemory(key); }
    }
    public async Task<JsonNode> Command(object request) { await queue.WaitAsync(); try { return await Task.Run(() => Call(handle, request)); } finally { queue.Release(); } }
    private static JsonNode Call(ulong core, object value) {
        var bytes=Encoding.UTF8.GetBytes(System.Text.Json.JsonSerializer.Serialize(value)); var request=fr_request_new(); ulong response=0;
        try {
            if (request==0) throw new InvalidOperationException("The native transport is busy.");
            for (var offset=0; offset<bytes.Length; offset+=8) { var count=Math.Min(8,bytes.Length-offset); ulong word=0; for(var i=0;i<count;i++) word|=(ulong)bytes[offset+i]<<(i*8); if(fr_request_push(request,word,(byte)count)==0) throw new InvalidOperationException("Request is too large."); }
            response=fr_dispatch(core,request); if(response==0) throw new InvalidOperationException("Native command failed.");
            var length=checked((int)fr_response_length(response)); var output=new byte[length];
            for(var offset=0;offset<length;offset+=8) { var word=fr_response_word(response,(ulong)(offset/8)); for(var i=0;i<Math.Min(8,length-offset);i++) output[offset+i]=(byte)(word>>(i*8)); }
            var result=JsonNode.Parse(output)!; if(result["ok"]?.GetValue<bool>()!=true) throw new InvalidOperationException(result["error"]?.GetValue<string>()??"Core command failed."); return result["result"]!.DeepClone();
        } finally { CryptographicOperations.ZeroMemory(bytes); fr_release(request); if(response!=0) fr_release(response); }
    }
    public void Dispose() { if(handle!=0) { fr_release(handle); handle=0; } queue.Dispose(); }
}
