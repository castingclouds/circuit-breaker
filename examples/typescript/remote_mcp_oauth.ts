/**
 * Remote MCP OAuth Demo
 *
 * This example demonstrates a complete multi-tenant remote MCP server setup:
 * 1. Server health check and status
 * 2. MCP app creation and installation
 * 3. OAuth provider registration (GitLab)
 * 4. Browser-based OAuth authentication flow
 * 5. GitLab API integration testing
 * 6. Project context discovery
 * 7. Issue management and user information retrieval
 *
 * ## Usage
 *
 * ```bash
 * # Run the complete MCP workflow demo
 * npm run remote-mcp-oauth demo full
 *
 * # Test GitLab integration only
 * npm run remote-mcp-oauth demo gitlab
 *
 * # Setup OAuth provider only
 * npm run remote-mcp-oauth demo setup-oauth
 *
 * # Interactive mode
 * npm run remote-mcp-oauth interactive
 * ```
 */

import { Command } from 'commander';
import axios, { AxiosResponse } from 'axios';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import * as crypto from 'crypto';
import * as jwt from 'jsonwebtoken';
const express = require('express');
import * as readline from 'readline';


// Types
interface CliConfig {
  serverUrl?: string;
  currentSession?: string;
  sessions: Record<string, SessionInfo>;
  oauthTokens: Record<string, OAuthTokenInfo>;
}

interface SessionInfo {
  sessionId: string;
  jwtToken: string;
  installationId?: string;
  appId: string;
  expiresAt: string;
  createdAt: string;
}

interface OAuthTokenInfo {
  providerType: string;
  accessToken: string;
  refreshToken?: string;
  expiresAt?: string;
  scope: string[];
  createdAt: string;
}

interface ServerStatus {
  status: string;
  version: string;
  uptime: string;
  activeSessions: number;
  registeredApps: number;
}

interface OAuthProviderRequest {
  providerType: string;
  clientId: string;
  clientSecret: string;
  authUrl: string;
  tokenUrl: string;
  scope: string[];
  redirectUri: string;
}

interface OAuthAuthResponse {
  authUrl: string;
  state: string;
}

interface CallbackResult {
  code?: string;
  state?: string;
  error?: string;
}

interface AppJWTClaims {
  iss: string; // app_id
  iat: number; // issued at
  exp: number; // expires at
  aud: string; // audience - "mcp-server"
}

class CliApp {
  private config: CliConfig;
  private configPath: string;
  private serverUrl: string;
  private verbose: boolean;

  constructor(serverUrl: string, configPath: string, verbose: boolean) {
    this.serverUrl = serverUrl;
    this.configPath = path.resolve(configPath.replace('~', os.homedir()));
    this.verbose = verbose;
    this.config = {
      sessions: {},
      oauthTokens: {}
    };
  }

  async init(): Promise<void> {
    try {
      const configData = await fs.readFile(this.configPath, 'utf8');
      this.config = JSON.parse(configData);
    } catch (error) {
      if (this.verbose) {
        console.log('No existing config found, creating new one');
      }
    }
  }

  async saveConfig(): Promise<void> {
    const configDir = path.dirname(this.configPath);
    await fs.mkdir(configDir, { recursive: true });
    await fs.writeFile(this.configPath, JSON.stringify(this.config, null, 2));
  }

  getCurrentToken(): string | undefined {
    if (this.config.currentSession) {
      return this.config.sessions[this.config.currentSession]?.jwtToken;
    }
    return undefined;
  }

  async makeRequest(
    method: string,
    path: string,
    body?: any
  ): Promise<AxiosResponse> {
    const url = `${this.serverUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json'
    };

    const token = this.getCurrentToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    if (this.verbose) {
      console.log(`${method} ${url}`);
      if (body) {
        console.log('Body:', JSON.stringify(body, null, 2));
      }
    }

    return axios({
      method: method.toLowerCase() as any,
      url,
      headers,
      data: body
    });
  }

  async makeRequestWithAppAuth(
    method: string,
    path: string,
    body?: any,
    appId?: string
  ): Promise<AxiosResponse> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json'
    };

    if (appId) {
      const appJwt = this.generateAppJwt(appId);
      headers['Authorization'] = `Bearer ${appJwt}`;
    }

    const url = `${this.serverUrl}${path}`;

    return axios({
      method: method.toLowerCase() as any,
      url,
      headers,
      data: body
    });
  }

  async startOAuthCallbackServer(port: number): Promise<{ server: any; result: Promise<CallbackResult> }> {
    const app = express();
    let resolveCallback: (result: CallbackResult) => void;
    
    const resultPromise = new Promise<CallbackResult>((resolve) => {
      resolveCallback = resolve;
    });

    app.get('/callback', (req: any, res: any) => {
      const { code, state, error, error_description } = req.query;
      
      const result: CallbackResult = {
        code: code as string,
        state: state as string,
        error: error as string
      };

      if (error) {
        res.send(`
          <html>
            <body>
              <h1>OAuth Error</h1>
              <p>Error: ${error}</p>
              <p>Description: ${error_description || 'No description provided'}</p>
              <p>You can close this window.</p>
            </body>
          </html>
        `);
      } else {
        res.send(`
          <html>
            <body>
              <h1>OAuth Success!</h1>
              <p>Authorization code received. You can close this window.</p>
              <script>setTimeout(() => window.close(), 2000);</script>
            </body>
          </html>
        `);
      }

      resolveCallback(result);
    });

    const server = app.listen(port, () => {
      console.log(`🌐 OAuth callback server started on http://localhost:${port}/callback`);
    });

    return { server, result: resultPromise };
  }

  generateTestKeys(): { privateKey: string; publicKey: string } {
    const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
      modulusLength: 2048,
      publicKeyEncoding: { type: 'spki', format: 'pem' },
      privateKeyEncoding: { type: 'pkcs8', format: 'pem' }
    });
    return { privateKey, publicKey };
  }

  generateAppJwt(appId: string): string {
    const { privateKey } = this.generateTestKeys();
    const now = Math.floor(Date.now() / 1000);
    
    const claims: AppJWTClaims = {
      iss: appId,
      iat: now,
      exp: now + 600, // 10 minutes
      aud: 'mcp-server'
    };

    return jwt.sign(claims, privateKey, { algorithm: 'RS256' });
  }

  async checkServerStatus(): Promise<ServerStatus> {
    console.log('🏥 Checking server health...');
    
    try {
      const response = await this.makeRequest('GET', '/health');
      const status: ServerStatus = response.data;
      
      console.log('✅ Server is healthy');
      console.log(`   Status: ${status.status}`);
      console.log(`   Version: ${status.version}`);
      console.log(`   Uptime: ${status.uptime}`);
      console.log(`   Active Sessions: ${status.activeSessions}`);
      console.log(`   Registered Apps: ${status.registeredApps}`);
      
      return status;
    } catch (error: any) {
      console.error(`❌ Server health check failed: ${error.message}`);
      throw error;
    }
  }

  async createApp(): Promise<string> {
    console.log('\n🔧 Creating MCP app...');
    
    const appId = `app_${crypto.randomBytes(8).toString('hex')}`;
    const { publicKey } = this.generateTestKeys();
    
    const appData = {
      app_id: appId,
      name: 'Remote MCP OAuth Demo',
      description: 'Demo app for OAuth workflow testing',
      public_key: publicKey,
      client_id: `client_${crypto.randomBytes(12).toString('hex')}`,
      client_secret: crypto.randomBytes(32).toString('hex'),
      app_type: 'remote_oauth'
    };

    try {
      const response = await this.makeRequest('POST', '/api/v1/mcp/apps', appData);
      
      if (response.status === 201) {
        console.log(`✅ App created successfully: ${appId}`);
        return appId;
      } else {
        throw new Error(`Failed to create app: ${response.status}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to create app: ${error.message}`);
      throw error;
    }
  }

  async installApp(appId: string): Promise<string> {
    console.log(`\n📦 Installing app: ${appId}`);
    
    const installationId = `inst_${crypto.randomBytes(8).toString('hex')}`;
    const projectContexts = this.detectGitProjectContext();
    
    const installationData = {
      app_id: appId,
      installation_id: installationId,
      account_type: 'user',
      permissions: {
        repositories: ['read', 'write'],
        issues: ['read', 'write'],
        pull_requests: ['read', 'write']
      },
      project_contexts: projectContexts
    };

    try {
      const response = await this.makeRequest('POST', '/api/v1/mcp/installations', installationData);
      
      if (response.status === 201) {
        console.log(`✅ App installed successfully: ${installationId}`);
        if (projectContexts.length > 0) {
          console.log(`   Detected project contexts: ${projectContexts.length}`);
        }
        return installationId;
      } else {
        throw new Error(`Failed to install app: ${response.status}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to install app: ${error.message}`);
      throw error;
    }
  }

  detectGitProjectContext(): any[] {
    // Simplified project context detection
    try {
      const { execSync } = require('child_process');
      const remoteUrl = execSync('git remote get-url origin', { encoding: 'utf8' }).trim();
      
      if (remoteUrl.includes('gitlab.com')) {
        const projectPath = this.extractGitlabProjectPath(remoteUrl);
        if (projectPath) {
          return [{
            type: 'gitlab_project',
            project_path: projectPath,
            remote_url: remoteUrl
          }];
        }
      }
    } catch (error) {
      // Git not available or not in a git repo
    }
    
    return [];
  }

  extractGitlabProjectPath(remoteUrl: string): string | null {
    const patterns = [
      /gitlab\.com[\/:]([^\/]+\/[^\/]+)\.git$/,
      /gitlab\.com[\/:]([^\/]+\/[^\/]+)$/
    ];
    
    for (const pattern of patterns) {
      const match = remoteUrl.match(pattern);
      if (match) {
        return match[1];
      }
    }
    
    return null;
  }

  async registerOAuthProvider(
    appId: string,
    ngrokUrl: string,
    clientId: string,
    clientSecret: string
  ): Promise<void> {
    console.log('\n🔐 Registering OAuth provider...');
    
    const providerData: OAuthProviderRequest = {
      providerType: 'gitlab',
      clientId,
      clientSecret,
      authUrl: 'https://gitlab.com/oauth/authorize',
      tokenUrl: 'https://gitlab.com/oauth/token',
      scope: ['read_user', 'api', 'read_repository'],
      redirectUri: `${ngrokUrl}/mcp/remote/oauth/callback`
    };

    try {
      const response = await this.makeRequestWithAppAuth(
        'POST',
        '/api/v1/oauth/providers',
        providerData,
        appId
      );
      
      if (response.status === 201) {
        console.log('✅ OAuth provider registered successfully');
        console.log(`   Provider: GitLab`);
        console.log(`   Redirect URI: ${providerData.redirectUri}`);
      } else {
        throw new Error(`Failed to register OAuth provider: ${response.status}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to register OAuth provider: ${error.message}`);
      throw error;
    }
  }

  async createAuthToken(appId: string, installationId: string): Promise<void> {
    console.log('\n🎫 Creating authentication token...');
    
    const sessionData = {
      installation_id: installationId,
      permissions: {
        repositories: ['read', 'write'],
        issues: ['read', 'write'],
        pull_requests: ['read', 'write']
      },
      project_contexts: ['default'],
      client_info: {
        name: 'Remote MCP OAuth Demo',
        version: '1.0.0',
        platform: 'typescript'
      }
    };

    try {
      const response = await this.makeRequestWithAppAuth(
        'POST',
        `/api/v1/mcp/installations/${installationId}/tokens`,
        sessionData,
        appId
      );
      
      if (response.status === 201) {
        const sessionInfo: SessionInfo = {
          sessionId: response.data.session_id || crypto.randomUUID(),
          jwtToken: response.data.token,
          installationId,
          appId,
          expiresAt: response.data.expires_at || new Date(Date.now() + 3600000).toISOString(),
          createdAt: new Date().toISOString()
        };

        this.config.sessions[sessionInfo.sessionId] = sessionInfo;
        this.config.currentSession = sessionInfo.sessionId;
        await this.saveConfig();

        console.log(`✅ Authentication token created: ${sessionInfo.sessionId}`);
        console.log(`   Expires at: ${sessionInfo.expiresAt}`);
      } else {
        throw new Error(`Failed to create auth token: ${response.status}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to create auth token: ${error.message}`);
      throw error;
    }
  }

  async oauthFlow(_appId: string, installationId: string): Promise<void> {
    console.log('\n🔄 Starting OAuth flow...');
    
    // Start callback server
    const { server, result } = await this.startOAuthCallbackServer(3001);
    
    try {
      // Get authorization URL
      const authResponse = await this.makeRequest('POST', '/api/v1/oauth/authorize', {
        provider_type: 'gitlab',
        installation_id: installationId,
        redirect_uri: 'http://localhost:3001/callback',
        scope: ['read_user', 'api', 'read_repository']
      });

      const authData: OAuthAuthResponse = authResponse.data;
      
      console.log('🌐 Opening browser for OAuth authorization...');
      console.log(`   Auth URL: ${authData.authUrl}`);
      
      // Open browser (simplified)
      const { exec } = require('child_process');
      exec(`open "${authData.authUrl}"`);
      
      console.log('⏳ Waiting for OAuth callback...');
      
      // Wait for callback
      const callbackResult = await result;
      
      if (callbackResult.error) {
        throw new Error(`OAuth error: ${callbackResult.error}`);
      }
      
      if (!callbackResult.code) {
        throw new Error('No authorization code received');
      }
      
      console.log('✅ OAuth authorization successful');
      console.log(`   Authorization code: ${callbackResult.code.substring(0, 20)}...`);
      
      // Exchange code for token
      const tokenResponse = await this.makeRequest('POST', '/api/v1/oauth/callback', {
        code: callbackResult.code,
        state: callbackResult.state,
        provider_type: 'gitlab'
      });
      
      if (tokenResponse.status === 200) {
        const tokenInfo: OAuthTokenInfo = {
          providerType: 'gitlab',
          accessToken: tokenResponse.data.access_token,
          refreshToken: tokenResponse.data.refresh_token,
          expiresAt: tokenResponse.data.expires_at,
          scope: tokenResponse.data.scope || ['read_user', 'api'],
          createdAt: new Date().toISOString()
        };

        this.config.oauthTokens['gitlab'] = tokenInfo;
        await this.saveConfig();

        console.log('✅ OAuth token exchange successful');
        console.log(`   Token expires: ${tokenInfo.expiresAt || 'Never'}`);
      }
      
    } finally {
      server.close();
    }
  }

  async testGitlabIntegration(): Promise<void> {
    console.log('\n🧪 Testing GitLab integration...');
    
    try {
      // Test MCP operations with GitLab tools
      const toolsResponse = await this.makeRequest('POST', '/mcp/v1/transport/http', {
        jsonrpc: '2.0',
        id: 'test-tools',
        method: 'tools/list',
        params: {}
      });
      
      if (toolsResponse.data.result?.tools) {
        const gitlabTools = toolsResponse.data.result.tools.filter((tool: any) => 
          tool.name.startsWith('gitlab_')
        );
        console.log(`✅ Found ${gitlabTools.length} GitLab tools`);
        
        // Test a GitLab tool
        if (gitlabTools.length > 0) {
          const listProjectsTool = gitlabTools.find((tool: any) => tool.name === 'gitlab_list_projects');
          if (listProjectsTool) {
            console.log('\n🔧 Testing gitlab_list_projects tool...');
            
            const toolResponse = await this.makeRequest('POST', '/mcp/v1/transport/http', {
              jsonrpc: '2.0',
              id: 'test-gitlab-projects',
              method: 'tools/call',
              params: {
                name: 'gitlab_list_projects',
                arguments: { per_page: 5 }
              }
            });
            
            if (toolResponse.data.result) {
              console.log('✅ GitLab projects tool executed successfully');
            }
          }
        }
      }
    } catch (error: any) {
      console.error(`❌ GitLab integration test failed: ${error.message}`);
    }
  }

  async demoBreakpoint(
    step: number,
    title: string,
    description: string,
    autoConfirm: boolean = false
  ): Promise<void> {
    console.log(`\n${'='.repeat(60)}`);
    console.log(`🔹 Step ${step}: ${title}`);
    console.log(description);
    console.log('='.repeat(60));

    if (!autoConfirm) {
      const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
      });

      await new Promise<void>((resolve) => {
        rl.question('\nPress Enter to continue...', () => {
          rl.close();
          resolve();
        });
      });
    } else {
      await new Promise(resolve => setTimeout(resolve, 1000));
    }
  }

  async runFullDemo(
    ngrokUrl?: string,
    gitlabClientId?: string,
    gitlabClientSecret?: string,
    autoConfirm: boolean = false
  ): Promise<void> {
    console.log('🚀 Starting Complete MCP OAuth Demo');
    console.log(`Server URL: ${this.serverUrl}`);

    try {
      // Get required parameters
      const finalNgrokUrl = ngrokUrl || await this.getOrPromptNgrokUrl();
      const [finalClientId, finalClientSecret] = await this.getOrPromptGitlabOAuth(
        gitlabClientId,
        gitlabClientSecret
      );

      // Step 1: Server Health Check
      await this.demoBreakpoint(
        1,
        'Server Health Check',
        'Checking if the MCP server is running and accessible.',
        autoConfirm
      );
      await this.checkServerStatus();

      // Step 2: App Creation
      await this.demoBreakpoint(
        2,
        'MCP App Creation',
        'Creating a new MCP application for OAuth workflow demonstration.',
        autoConfirm
      );
      const appId = await this.createApp();

      // Step 3: App Installation
      await this.demoBreakpoint(
        3,
        'App Installation',
        'Installing the app and detecting project context from git repository.',
        autoConfirm
      );
      const installationId = await this.installApp(appId);

      // Step 4: OAuth Provider Registration
      await this.demoBreakpoint(
        4,
        'OAuth Provider Registration',
        'Registering GitLab as an OAuth provider with the MCP server.',
        autoConfirm
      );
      await this.registerOAuthProvider(appId, finalNgrokUrl, finalClientId, finalClientSecret);

      // Step 5: Authentication Token Creation
      await this.demoBreakpoint(
        5,
        'Authentication Token Creation',
        'Creating session tokens for secure API access.',
        autoConfirm
      );
      await this.createAuthToken(appId, installationId);

      // Step 6: OAuth Flow
      await this.demoBreakpoint(
        6,
        'OAuth Authorization Flow',
        'Performing browser-based OAuth flow with GitLab.',
        autoConfirm
      );
      await this.oauthFlow(appId, installationId);

      // Step 7: GitLab Integration Testing
      await this.demoBreakpoint(
        7,
        'GitLab Integration Testing',
        'Testing GitLab API integration through MCP tools.',
        autoConfirm
      );
      await this.testGitlabIntegration();

      console.log('\n🎉 Complete MCP OAuth Demo finished successfully!');
      console.log('\n📋 Summary:');
      console.log(`   • App ID: ${appId}`);
      console.log(`   • Installation ID: ${installationId}`);
      console.log(`   • OAuth Provider: GitLab`);
      console.log(`   • NgRok URL: ${finalNgrokUrl}`);

    } catch (error: any) {
      console.error(`\n❌ Demo failed: ${error.message}`);
      throw error;
    }
  }

  async getOrPromptNgrokUrl(ngrokUrl?: string): Promise<string> {
    if (ngrokUrl) {
      return ngrokUrl;
    }

    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout
    });

    return new Promise((resolve) => {
      rl.question('Enter your NgRok URL (e.g., https://abc123.ngrok.io): ', (answer) => {
        rl.close();
        resolve(answer.trim());
      });
    });
  }

  async getOrPromptGitlabOAuth(
    clientId?: string,
    clientSecret?: string
  ): Promise<[string, string]> {
    if (clientId && clientSecret) {
      return [clientId, clientSecret];
    }

    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout
    });

    const question = (prompt: string): Promise<string> => {
      return new Promise((resolve) => {
        rl.question(prompt, resolve);
      });
    };

    try {
      const finalClientId = clientId || await question('Enter GitLab OAuth Client ID: ');
      const finalClientSecret = clientSecret || await question('Enter GitLab OAuth Client Secret: ');
      
      return [finalClientId.trim(), finalClientSecret.trim()];
    } finally {
      rl.close();
    }
  }

  async runInteractive(): Promise<void> {
    console.log('🎮 Interactive Remote MCP OAuth Mode');
    
    const rl = readline.createInterface({
      input: process.stdin,
      output: process.stdout
    });

    const question = (prompt: string): Promise<string> => {
      return new Promise((resolve) => {
        rl.question(prompt, resolve);
      });
    };

    try {
      while (true) {
        console.log('\n📋 Available actions:');
        console.log('1. Run full demo');
        console.log('2. Check server status');
        console.log('3. Test GitLab integration');
        console.log('4. List sessions');
        console.log('5. Exit');

        const choice = await question('\nSelect an action (1-5): ');

        switch (choice) {
          case '1':
            await this.runFullDemo();
            break;
          case '2':
            await this.checkServerStatus();
            break;
          case '3':
            await this.testGitlabIntegration();
            break;
          case '4':
            await this.listSessions();
            break;
          case '5':
            console.log('👋 Goodbye!');
            rl.close();
            return;
          default:
            console.log('❌ Invalid choice. Please select 1-5.');
        }
      }
    } finally {
      rl.close();
    }
  }

  async listSessions(): Promise<void> {
    console.log('\n📋 Active Sessions:');
    
    if (Object.keys(this.config.sessions).length === 0) {
      console.log('No active sessions found.');
      return;
    }

    for (const [sessionId, session] of Object.entries(this.config.sessions)) {
      const expiresAt = new Date(session.expiresAt);
      const isExpired = expiresAt < new Date();
      const status = isExpired ? '❌ Expired' : '✅ Active';
      
      console.log(`\n  Session: ${sessionId}`);
      console.log(`    App ID: ${session.appId}`);
      console.log(`    Installation: ${session.installationId}`);
      console.log(`    Status: ${status}`);
      console.log(`    Expires: ${session.expiresAt}`);
      console.log(`    Created: ${session.createdAt}`);
    }
  }
}

// CLI Setup
const program = new Command();

program
  .name('remote-mcp-oauth')
  .description('Remote Multi-Context Protocol Server with OAuth Demo')
  .version('1.0.0');

program
  .option('--server-url <url>', 'Server URL', process.env.MCP_SERVER_URL || 'http://localhost:8080')
  .option('--config <path>', 'Configuration file path', '~/.remote-mcp-oauth.json')
  .option('-v, --verbose', 'Verbose output', false);

const demoCommand = program
  .command('demo')
  .description('Run MCP demos');

demoCommand
  .command('full')
  .description('Run the complete MCP workflow demo')
  .option('--ngrok-url <url>', 'NgRok URL for the MCP server', process.env.NGROK_URL)
  .option('--gitlab-client-id <id>', 'GitLab OAuth Client ID', process.env.GITLAB_CLIENT_ID)
  .option('--gitlab-client-secret <secret>', 'GitLab OAuth Client Secret', process.env.GITLAB_CLIENT_SECRET)
  .option('--auto-confirm', 'Skip confirmation prompts')
  .action(async (options) => {
    const app = new CliApp(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await app.init();
    await app.runFullDemo(
      options.ngrokUrl,
      options.gitlabClientId,
      options.gitlabClientSecret,
      options.autoConfirm
    );
  });

demoCommand
  .command('gitlab')
  .description('Test GitLab integration only')
  .action(async () => {
    const app = new CliApp(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await app.init();
    await app.testGitlabIntegration();
  });

program
  .command('interactive')
  .description('Interactive mode')
  .action(async () => {
    const app = new CliApp(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await app.init();
    await app.runInteractive();
  });

program
  .command('status')
  .description('Check server status')
  .action(async () => {
    const app = new CliApp(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await app.checkServerStatus();
  });

// Main execution
if (require.main === module) {
  program.parse();
}

export { CliApp }; 