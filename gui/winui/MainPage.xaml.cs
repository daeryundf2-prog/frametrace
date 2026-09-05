using System.Diagnostics;
using System.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace FrameTrace_Shell;

public sealed partial class MainPage : Page
{
	string? _enginePath;
	string? _appPath;

	public MainPage()
	{
		InitializeComponent();
		RefreshEnginePaths();
	}

	void RefreshEngine_Click(object sender, RoutedEventArgs e) => RefreshEnginePaths();

	void RefreshEnginePaths()
	{
		_enginePath = EngineLocator.Find("frametrace.exe");
		_appPath = EngineLocator.Find("frametrace-app.exe") ?? _enginePath;
		EngineStatus.Text = _enginePath is null
			? "엔진을 찾지 못했습니다. target/release 또는 PATH에 frametrace.exe를 두세요."
			: $"엔진: {_enginePath}";
	}

	async void LaunchWorkstation_Click(object sender, RoutedEventArgs e)
	{
		if (_appPath is null)
		{
			Append("frametrace-app.exe / frametrace.exe 없음");
			return;
		}
		try
		{
			Process.Start(new ProcessStartInfo
			{
				FileName = _appPath,
				UseShellExecute = true,
			});
			Append($"started {_appPath}");
		}
		catch (Exception ex)
		{
			Append(ex.Message);
		}
		await Task.CompletedTask;
	}

	async void MakeReview_Click(object sender, RoutedEventArgs e) =>
		await RunEngineAsync("make-review", CasePathBox.Text.Trim());

	async void MakeReport_Click(object sender, RoutedEventArgs e) =>
		await RunEngineAsync("make-report", CasePathBox.Text.Trim());

	async void Anomalies_Click(object sender, RoutedEventArgs e) =>
		await RunEngineAsync("qa", "anomalies", CasePathBox.Text.Trim());

	async Task RunEngineAsync(params string[] args)
	{
		if (_enginePath is null)
		{
			Append("frametrace.exe 없음");
			return;
		}
		if (string.IsNullOrWhiteSpace(CasePathBox.Text))
		{
			Append("케이스 폴더를 입력하세요.");
			return;
		}
		Append($"> frametrace {string.Join(' ', args)}");
		try
		{
			var psi = new ProcessStartInfo
			{
				FileName = _enginePath,
				RedirectStandardOutput = true,
				RedirectStandardError = true,
				UseShellExecute = false,
				CreateNoWindow = true,
				StandardOutputEncoding = Encoding.UTF8,
				StandardErrorEncoding = Encoding.UTF8,
			};
			foreach (var arg in args)
			{
				psi.ArgumentList.Add(arg);
			}
			using var process = Process.Start(psi)
				?? throw new InvalidOperationException("failed to start frametrace");
			var stdout = await process.StandardOutput.ReadToEndAsync();
			var stderr = await process.StandardError.ReadToEndAsync();
			await process.WaitForExitAsync();
			if (!string.IsNullOrWhiteSpace(stdout)) Append(stdout.TrimEnd());
			if (!string.IsNullOrWhiteSpace(stderr)) Append(stderr.TrimEnd());
			Append($"exit {process.ExitCode}");
		}
		catch (Exception ex)
		{
			Append(ex.Message);
		}
	}

	void Append(string line)
	{
		OutputBox.Text = string.IsNullOrEmpty(OutputBox.Text)
			? line
			: OutputBox.Text + Environment.NewLine + line;
	}
}

static class EngineLocator
{
	public static string? Find(string fileName)
	{
		var candidates = new List<string>();
		var baseDir = AppContext.BaseDirectory;
		candidates.Add(Path.Combine(baseDir, fileName));
		candidates.Add(Path.Combine(baseDir, "tools", "bin", fileName));
		var cwd = Environment.CurrentDirectory;
		candidates.Add(Path.Combine(cwd, fileName));
		candidates.Add(Path.Combine(cwd, "target", "release", fileName));
		candidates.Add(Path.Combine(cwd, "..", "..", "..", "..", "target", "release", fileName));
		var pathEnv = Environment.GetEnvironmentVariable("PATH") ?? "";
		foreach (var dir in pathEnv.Split(Path.PathSeparator, StringSplitOptions.RemoveEmptyEntries))
		{
			candidates.Add(Path.Combine(dir.Trim('"'), fileName));
		}
		foreach (var path in candidates)
		{
			try
			{
				var full = Path.GetFullPath(path);
				if (File.Exists(full)) return full;
			}
			catch
			{
				// ignore invalid candidates
			}
		}
		return null;
	}
}
