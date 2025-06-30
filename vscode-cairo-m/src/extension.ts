import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
  console.log("Cairo-M: activate() called");

  const outputChannel = vscode.window.createOutputChannel("Cairo-M Extension");
  outputChannel.appendLine("Cairo-M extension activating...");
  outputChannel.show();

  try {
    // Get the language server executable path
    const serverPath = await getServerPath(context);
    outputChannel.appendLine(`Looking for language server...`);

    if (!serverPath) {
      outputChannel.appendLine("Language server not found!");
      vscode.window.showErrorMessage(
        "Cairo-M language server not found. Please build it or set the path in settings.",
      );
      return;
    }

    outputChannel.appendLine(`Found language server at: ${serverPath}`);

    // Server executable options
    const serverExecutable: Executable = {
      command: serverPath,
      args: [],
      options: {
        env: process.env,
      },
    };

    const serverOptions: ServerOptions = {
      run: serverExecutable,
      debug: serverExecutable,
    };

    // Enable trace based on configuration
    const config = vscode.workspace.getConfiguration("cairo-m");
    const trace = config.get<string>("trace.server", "off");

    // Client options
    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ scheme: "file", language: "cairo-m" }],
      synchronize: {
        // Synchronize the setting section 'cairo-m' to the server
        configurationSection: "cairo-m",
        // Notify the server about file changes to '.cm' files contained in the workspace
        fileEvents: vscode.workspace.createFileSystemWatcher("**/*.cm"),
      },
      outputChannelName: "Cairo-M Language Server",
      traceOutputChannel: vscode.window.createOutputChannel(
        "Cairo-M Language Server Trace",
      ),
    };

    // Create the language client and start it
    client = new LanguageClient(
      "cairo-m-ls",
      "Cairo-M Language Server",
      serverOptions,
      clientOptions,
    );

    // Set trace level
    await client.setTrace(trace as any);

    // Start the client. This will also launch the server
    outputChannel.appendLine("Starting language client...");
    try {
      await client.start();
      outputChannel.appendLine("Cairo-M language server started successfully!");
    } catch (error) {
      outputChannel.appendLine(`Failed to start language server: ${error}`);
      vscode.window.showErrorMessage(
        `Failed to start Cairo-M language server: ${error}`,
      );
      return;
    }

    // Register commands
    context.subscriptions.push(
      vscode.commands.registerCommand("cairo-m.restartServer", async () => {
        await restartServer(context);
      }),
    );

    outputChannel.appendLine("Extension activation complete!");
  } catch (error) {
    outputChannel.appendLine(`Failed during activation: ${error}`);
    console.error("Cairo-M: activation error:", error);
    throw error;
  }
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

async function getServerPath(
  context: vscode.ExtensionContext,
): Promise<string | undefined> {
  const outputChannel = vscode.window.createOutputChannel(
    "Cairo-M Path Discovery",
  );

  // First check if user has specified a custom path
  const config = vscode.workspace.getConfiguration("cairo-m");
  const customPath = config.get<string>("languageServer.path");

  outputChannel.appendLine(
    `Custom path from settings: ${customPath || "not set"}`,
  );

  if (customPath && fs.existsSync(customPath)) {
    outputChannel.appendLine(`Using custom path: ${customPath}`);
    return customPath;
  }

  // Try to find the bundled language server
  const possiblePaths = [
    // Development path (when running from source)
    path.join(context.extensionPath, "..", "target", "release", "cairo-m-ls"),
    path.join(context.extensionPath, "..", "target", "debug", "cairo-m-ls"),
    // Bundled path (when packaged)
    path.join(context.extensionPath, "server", "cairo-m-ls"),
    // Platform-specific extensions for Windows
    path.join(
      context.extensionPath,
      "..",
      "target",
      "release",
      "cairo-m-ls.exe",
    ),
    path.join(context.extensionPath, "..", "target", "debug", "cairo-m-ls.exe"),
    path.join(context.extensionPath, "server", "cairo-m-ls.exe"),
  ];

  outputChannel.appendLine("Checking possible paths:");
  for (const serverPath of possiblePaths) {
    outputChannel.appendLine(`  Checking: ${serverPath}`);
    if (fs.existsSync(serverPath)) {
      outputChannel.appendLine(`  ✓ Found at: ${serverPath}`);
      // Check if it's executable
      try {
        fs.accessSync(serverPath, fs.constants.X_OK);
        outputChannel.appendLine(`  ✓ File is executable`);
      } catch {
        outputChannel.appendLine(`  ✗ File is not executable - fixing...`);
        try {
          fs.chmodSync(serverPath, 0o755);
          outputChannel.appendLine(`  ✓ Made file executable`);
        } catch (e) {
          outputChannel.appendLine(`  ✗ Failed to make executable: ${e}`);
        }
      }
      outputChannel.show();
      return serverPath;
    } else {
      outputChannel.appendLine(`  ✗ Not found`);
    }
  }

  outputChannel.appendLine(
    "No language server found in any expected location!",
  );
  outputChannel.show();
  return undefined;
}

async function restartServer(context: vscode.ExtensionContext) {
  if (client) {
    await client.stop();
  }

  await activate(context);
  vscode.window.showInformationMessage("Cairo-M language server restarted");
}
