/**
 * Secure Agent JWT Authentication Demo
 *
 * This example demonstrates the complete JWT authentication flow for MCP agents,
 * showing how to securely authenticate with the Circuit Breaker MCP server using
 * the GitHub Apps-inspired authentication model.
 *
 * ## Authentication Flow
 *
 * 1. **App Registration**: Create MCP app with RSA key pair
 * 2. **App Installation**: Install app to organization/user
 * 3. **JWT Generation**: Create short-lived app JWT using private key
 * 4. **Session Token**: Exchange app JWT for session access token
 * 5. **API Requests**: Use session token for MCP operations
 * 6. **Token Refresh**: Handle token expiration and renewal
 *
 * ## Usage
 *
 * ```bash
 * # Run the complete JWT authentication demo
 * npm run secure-agent-jwt demo full
 *
 * # Test JWT generation only
 * npm run secure-agent-jwt demo jwt-only
 *
 * # Test session management
 * npm run secure-agent-jwt demo session-mgmt
 *
 * # Interactive mode
 * npm run secure-agent-jwt interactive
 * ```
 */

import { Command } from 'commander';
import * as jwt from 'jsonwebtoken';
import * as crypto from 'crypto';
import axios, { AxiosResponse } from 'axios';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';

import * as readline from 'readline';

// Types
interface JwtConfig {
  serverUrl: string;
  currentApp?: AppInfo;
  currentSession?: SessionInfo;
  apps: Record<string, AppInfo>;
  sessions: Record<string, SessionInfo>;
}

interface AppInfo {
  appId: string;
  name: string;
  description: string;
  privateKey: string;
  publicKey: string;
  clientId: string;
  clientSecret: string;
  createdAt: string;
  installations: InstallationInfo[];
}

interface InstallationInfo {
  installationId: string;
  appId: string;
  accountType: string;
  createdAt: string;
}

interface SessionInfo {
  sessionId: string;
  appId: string;
  installationId: string;
  accessToken: string;
  expiresAt: string;
  createdAt: string;
  permissions: any;
}

interface AppJwtClaims {
  iss: string; // App ID (issuer)
  iat: number; // Issued at
  exp: number; // Expires at
  aud: string; // Audience - "circuit-breaker-mcp"
}

interface SessionJwtClaims {
  iss: string; // "circuit-breaker-mcp"
  sub: string; // Installation ID
  appId: string;
  installationId: string;
  permissions: any;
  iat: number; // Issued at
  exp: number; // Expires at
  jti: string; // JWT ID (unique token identifier)
}

class JwtDemo {
  private config: JwtConfig;
  private configPath: string;
  private verbose: boolean;

  constructor(serverUrl: string, configPath: string, verbose: boolean) {
    this.configPath = path.resolve(configPath.replace('~', os.homedir()));
    this.verbose = verbose;
    this.config = {
      serverUrl,
      apps: {},
      sessions: {}
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

  generateRsaKeys(): { privateKey: string; publicKey: string } {
    const { publicKey, privateKey } = crypto.generateKeyPairSync('rsa', {
      modulusLength: 2048,
      publicKeyEncoding: {
        type: 'spki',
        format: 'pem'
      },
      privateKeyEncoding: {
        type: 'pkcs8',
        format: 'pem'
      }
    });

    return { privateKey, publicKey };
  }

  generateAppJwt(appId: string, privateKeyPem: string): string {
    const now = Math.floor(Date.now() / 1000);
    const claims: AppJwtClaims = {
      iss: appId,
      iat: now,
      exp: now + 600, // 10 minutes
      aud: 'circuit-breaker-mcp'
    };

    return jwt.sign(claims, privateKeyPem, { algorithm: 'RS256' });
  }

  validateAppJwt(token: string, publicKeyPem: string): AppJwtClaims {
    return jwt.verify(token, publicKeyPem, { 
      algorithms: ['RS256'],
      audience: 'circuit-breaker-mcp'
    }) as AppJwtClaims;
  }

  async makeRequest(
    method: string,
    path: string,
    body?: any,
    authToken?: string
  ): Promise<AxiosResponse> {
    const url = `${this.config.serverUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json'
    };

    if (authToken) {
      headers['Authorization'] = `Bearer ${authToken}`;
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

  async createApp(name: string, description: string): Promise<AppInfo> {
    console.log(`\n🔧 Creating MCP app: ${name}`);
    
    const { privateKey, publicKey } = this.generateRsaKeys();
    const appId = `app_${crypto.randomBytes(8).toString('hex')}`;
    const clientId = `client_${crypto.randomBytes(12).toString('hex')}`;
    const clientSecret = crypto.randomBytes(32).toString('hex');

    const appData = {
      app_id: appId,
      name,
      description,
      public_key: publicKey,
      client_id: clientId,
      client_secret: clientSecret,
      app_type: 'agent'
    };

    try {
      const response = await this.makeRequest('POST', '/api/v1/mcp/apps', appData);
      
      if (response.status === 201) {
        const appInfo: AppInfo = {
          appId,
          name,
          description,
          privateKey,
          publicKey,
          clientId,
          clientSecret,
          createdAt: new Date().toISOString(),
          installations: []
        };

        this.config.apps[appId] = appInfo;
        this.config.currentApp = appInfo;
        await this.saveConfig();

        console.log(`✅ App created successfully: ${appId}`);
        return appInfo;
      } else {
        throw new Error(`Failed to create app: ${response.status} ${response.statusText}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to create app: ${error.message}`);
      throw error;
    }
  }

  async installApp(appId: string): Promise<InstallationInfo> {
    console.log(`\n📦 Installing app: ${appId}`);
    
    const installationId = `inst_${crypto.randomBytes(8).toString('hex')}`;
    const installationData = {
      app_id: appId,
      installation_id: installationId,
      account_type: 'user',
      permissions: {
        repositories: ['read', 'write'],
        issues: ['read', 'write'],
        pull_requests: ['read', 'write']
      }
    };

    try {
      const response = await this.makeRequest('POST', '/api/v1/mcp/installations', installationData);
      
      if (response.status === 201) {
        const installation: InstallationInfo = {
          installationId,
          appId,
          accountType: 'user',
          createdAt: new Date().toISOString()
        };

        if (this.config.apps[appId]) {
          this.config.apps[appId].installations.push(installation);
          await this.saveConfig();
        }

        console.log(`✅ App installed successfully: ${installationId}`);
        return installation;
      } else {
        throw new Error(`Failed to install app: ${response.status} ${response.statusText}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to install app: ${error.message}`);
      throw error;
    }
  }

  async createSessionToken(appId: string, installationId: string): Promise<SessionInfo> {
    console.log(`\n🎫 Creating session token for installation: ${installationId}`);
    
    const app = this.config.apps[appId];
    if (!app) {
      throw new Error(`App not found: ${appId}`);
    }

    // Generate app JWT
    const appJwt = this.generateAppJwt(appId, app.privateKey);
    
    const sessionData = {
      installation_id: installationId,
      permissions: {
        repositories: ['read', 'write'],
        issues: ['read', 'write'],
        pull_requests: ['read', 'write']
      },
      project_contexts: ['default'],
      client_info: {
        name: 'Secure Agent JWT Demo',
        version: '1.0.0',
        platform: 'typescript'
      }
    };

    try {
      const response = await this.makeRequest(
        'POST',
        `/api/v1/mcp/installations/${installationId}/tokens`,
        sessionData,
        appJwt
      );
      
      if (response.status === 201) {
        const sessionInfo: SessionInfo = {
          sessionId: response.data.session_id || crypto.randomUUID(),
          appId,
          installationId,
          accessToken: response.data.token,
          expiresAt: response.data.expires_at || new Date(Date.now() + 3600000).toISOString(),
          createdAt: new Date().toISOString(),
          permissions: sessionData.permissions
        };

        this.config.sessions[sessionInfo.sessionId] = sessionInfo;
        this.config.currentSession = sessionInfo;
        await this.saveConfig();

        console.log(`✅ Session token created: ${sessionInfo.sessionId}`);
        console.log(`   Expires at: ${sessionInfo.expiresAt}`);
        return sessionInfo;
      } else {
        throw new Error(`Failed to create session token: ${response.status} ${response.statusText}`);
      }
    } catch (error: any) {
      console.error(`❌ Failed to create session token: ${error.message}`);
      throw error;
    }
  }

  async testMcpOperations(sessionToken: string): Promise<void> {
    console.log('\n🧪 Testing MCP operations with session token...');
    
    try {
      // Test 1: List tools
      console.log('\n1. Testing tools/list...');
      const toolsResponse = await this.makeRequest(
        'POST',
        '/mcp/v1/transport/http',
        {
          jsonrpc: '2.0',
          id: 'test-1',
          method: 'tools/list',
          params: {}
        },
        sessionToken
      );
      
      if (toolsResponse.data.result) {
        console.log(`✅ Found ${toolsResponse.data.result.tools?.length || 0} tools`);
      }

      // Test 2: List prompts
      console.log('\n2. Testing prompts/list...');
      const promptsResponse = await this.makeRequest(
        'POST',
        '/mcp/v1/transport/http',
        {
          jsonrpc: '2.0',
          id: 'test-2',
          method: 'prompts/list',
          params: {}
        },
        sessionToken
      );
      
      if (promptsResponse.data.result) {
        console.log(`✅ Found ${promptsResponse.data.result.prompts?.length || 0} prompts`);
      }

      // Test 3: List resources
      console.log('\n3. Testing resources/list...');
      const resourcesResponse = await this.makeRequest(
        'POST',
        '/mcp/v1/transport/http',
        {
          jsonrpc: '2.0',
          id: 'test-3',
          method: 'resources/list',
          params: {}
        },
        sessionToken
      );
      
      if (resourcesResponse.data.result) {
        console.log(`✅ Found ${resourcesResponse.data.result.resources?.length || 0} resources`);
      }

      console.log('\n✅ All MCP operations completed successfully!');
    } catch (error: any) {
      console.error(`❌ MCP operations failed: ${error.message}`);
      throw error;
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

  async runFullDemo(autoConfirm: boolean = false): Promise<void> {
    console.log('🚀 Starting Complete JWT Authentication Demo');
    console.log(`Server URL: ${this.config.serverUrl}`);

    try {
      // Step 1: Environment Setup
      await this.demoBreakpoint(
        1,
        'Environment Setup',
        'Setting up the JWT authentication environment and checking server connectivity.',
        autoConfirm
      );

      // Step 2: App Creation
      await this.demoBreakpoint(
        2,
        'App Creation',
        'Creating a new MCP application with RSA key pair for secure authentication.',
        autoConfirm
      );

      const app = await this.createApp(
        'JWT Demo App',
        'Demonstration app for JWT authentication flow'
      );

      // Step 3: App Installation
      await this.demoBreakpoint(
        3,
        'App Installation',
        'Installing the app to create an installation context for authentication.',
        autoConfirm
      );

      const installation = await this.installApp(app.appId);

      // Step 4: JWT Generation
      await this.demoBreakpoint(
        4,
        'JWT Generation',
        'Generating short-lived app JWT using RSA private key for server authentication.',
        autoConfirm
      );

      const appJwt = this.generateAppJwt(app.appId, app.privateKey);
      console.log(`✅ Generated app JWT (expires in 10 minutes)`);
      console.log(`   JWT: ${appJwt.substring(0, 50)}...`);

      // Step 5: Session Token Creation
      await this.demoBreakpoint(
        5,
        'Session Token Creation',
        'Exchanging app JWT for a session access token with specific permissions.',
        autoConfirm
      );

      const session = await this.createSessionToken(app.appId, installation.installationId);

      // Step 6: MCP Operations
      await this.demoBreakpoint(
        6,
        'MCP Operations',
        'Testing MCP protocol operations using the session token.',
        autoConfirm
      );

      await this.testMcpOperations(session.accessToken);

      // Step 7: Token Validation
      await this.demoBreakpoint(
        7,
        'Token Validation',
        'Validating JWT tokens and checking their claims and expiration.',
        autoConfirm
      );

      try {
        const claims = this.validateAppJwt(appJwt, app.publicKey);
        console.log('✅ JWT validation successful');
        console.log(`   Issuer: ${claims.iss}`);
        console.log(`   Audience: ${claims.aud}`);
        console.log(`   Expires: ${new Date(claims.exp * 1000).toISOString()}`);
      } catch (error) {
        console.log('❌ JWT validation failed (expected if expired)');
      }

      // Step 8: Session Management
      await this.demoBreakpoint(
        8,
        'Session Management',
        'Demonstrating session lifecycle management and token refresh patterns.',
        autoConfirm
      );

      await this.demonstrateSessionManagement(session);

      console.log('\n🎉 Complete JWT Authentication Demo finished successfully!');
      console.log('\n📋 Summary:');
      console.log(`   • App ID: ${app.appId}`);
      console.log(`   • Installation ID: ${installation.installationId}`);
      console.log(`   • Session ID: ${session.sessionId}`);
      console.log(`   • Session expires: ${session.expiresAt}`);

    } catch (error: any) {
      console.error(`\n❌ Demo failed: ${error.message}`);
      throw error;
    }
  }

  async demonstrateSessionManagement(session: SessionInfo): Promise<void> {
    console.log('\n🔄 Session Management Demo');
    
    const expiresAt = new Date(session.expiresAt);
    const now = new Date();
    const timeUntilExpiry = expiresAt.getTime() - now.getTime();
    
    if (timeUntilExpiry > 5 * 60 * 1000) { // More than 5 minutes
      console.log(`Session is still valid for ${Math.floor(timeUntilExpiry / 60000)} minutes`);
      console.log('In a real application, you would refresh the token before it expires.');
      
      // Simulate token refresh
      console.log('\n🔄 Simulating token refresh...');
      const newSession = await this.createSessionToken(session.appId, session.installationId);
      console.log('✅ New session token created');
      console.log(`   New expiry: ${newSession.expiresAt}`);
    } else {
      console.log('⚠️  Session is close to expiry or expired');
      console.log('Refreshing session token...');
      
      const newSession = await this.createSessionToken(session.appId, session.installationId);
      console.log('✅ Session refreshed successfully');
      console.log(`   New expiry: ${newSession.expiresAt}`);
    }
  }

  async runJwtOnlyDemo(): Promise<void> {
    console.log('🔐 JWT Generation and Validation Demo');
    
    // Generate keys
    const { privateKey, publicKey } = this.generateRsaKeys();
    console.log('✅ Generated RSA key pair');
    
    // Generate JWT
    const appId = 'demo-app-123';
    const token = this.generateAppJwt(appId, privateKey);
    console.log('✅ Generated JWT token');
    console.log(`   Token: ${token.substring(0, 50)}...`);
    
    // Validate JWT
    try {
      const claims = this.validateAppJwt(token, publicKey);
      console.log('✅ JWT validation successful');
      console.log(`   Claims:`, claims);
    } catch (error: any) {
      console.error(`❌ JWT validation failed: ${error.message}`);
    }
  }

  async runSessionMgmtDemo(): Promise<void> {
    console.log('📋 Session Management Demo');
    
    console.log('\nActive sessions:');
    for (const [sessionId, session] of Object.entries(this.config.sessions)) {
      const expiresAt = new Date(session.expiresAt);
      const isExpired = expiresAt < new Date();
      const status = isExpired ? '❌ Expired' : '✅ Active';
      
      console.log(`  ${sessionId}: ${status} (expires: ${session.expiresAt})`);
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

  async runInteractive(): Promise<void> {
    console.log('🎮 Interactive JWT Demo Mode');
    
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
        console.log('2. JWT generation only');
        console.log('3. Session management');
        console.log('4. List sessions');
        console.log('5. Exit');

        const choice = await question('\nSelect an action (1-5): ');

        switch (choice) {
          case '1':
            await this.runFullDemo();
            break;
          case '2':
            await this.runJwtOnlyDemo();
            break;
          case '3':
            await this.runSessionMgmtDemo();
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

  async generateAndSaveKeys(): Promise<void> {
    const { privateKey, publicKey } = this.generateRsaKeys();
    
    await fs.writeFile('private_key.pem', privateKey);
    await fs.writeFile('public_key.pem', publicKey);
    
    console.log('✅ RSA key pair generated and saved:');
    console.log('   • private_key.pem');
    console.log('   • public_key.pem');
  }
}

// CLI Setup
const program = new Command();

program
  .name('secure-agent-jwt')
  .description('Secure Agent JWT Authentication Demo')
  .version('1.0.0');

program
  .option('--server-url <url>', 'MCP Server URL', process.env.MCP_SERVER_URL || 'http://localhost:3000')
  .option('--config <path>', 'Configuration file path', '~/.secure-agent-jwt.json')
  .option('-v, --verbose', 'Verbose output', false);

const demoCommand = program
  .command('demo')
  .description('Run authentication demos');

demoCommand
  .command('full')
  .description('Complete JWT authentication flow')
  .option('--auto-confirm', 'Skip confirmation prompts')
  .action(async (options) => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await demo.init();
    await demo.runFullDemo(options.autoConfirm);
  });

demoCommand
  .command('jwt-only')
  .description('JWT generation and validation only')
  .action(async () => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await demo.init();
    await demo.runJwtOnlyDemo();
  });

demoCommand
  .command('session-mgmt')
  .description('Session management demo')
  .action(async () => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await demo.init();
    await demo.runSessionMgmtDemo();
  });

program
  .command('interactive')
  .description('Interactive mode')
  .action(async () => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await demo.init();
    await demo.runInteractive();
  });

const jwtCommand = program
  .command('jwt')
  .description('JWT operations');

jwtCommand
  .command('generate')
  .description('Generate app JWT')
  .requiredOption('-a, --app-id <id>', 'App ID')
  .requiredOption('-k, --private-key <path>', 'Private key file path')
  .action(async (options) => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    const privateKey = await fs.readFile(options.privateKey, 'utf8');
    const token = demo.generateAppJwt(options.appId, privateKey);
    console.log('Generated JWT:', token);
  });

jwtCommand
  .command('validate')
  .description('Validate JWT token')
  .requiredOption('-t, --token <token>', 'JWT token to validate')
  .requiredOption('-k, --public-key <path>', 'Public key file path')
  .action(async (options) => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    const publicKey = await fs.readFile(options.publicKey, 'utf8');
    try {
      const claims = demo.validateAppJwt(options.token, publicKey);
      console.log('✅ JWT is valid');
      console.log('Claims:', claims);
    } catch (error: any) {
      console.error('❌ JWT validation failed:', error.message);
    }
  });

jwtCommand
  .command('generate-keys')
  .description('Generate RSA key pair')
  .action(async () => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await demo.generateAndSaveKeys();
  });

const sessionCommand = program
  .command('session')
  .description('Session management');

sessionCommand
  .command('list')
  .description('List active sessions')
  .action(async () => {
    const demo = new JwtDemo(
      program.opts().serverUrl,
      program.opts().config,
      program.opts().verbose
    );
    await demo.init();
    await demo.listSessions();
  });

// Main execution
if (require.main === module) {
  program.parse();
}

export { JwtDemo }; 