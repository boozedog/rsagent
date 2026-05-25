{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.services.rsagent;

  toolType = lib.types.submodule {
    options = {
      name = lib.mkOption {
        type = lib.types.str;
        description = "Tool name exposed to the model.";
      };
      description = lib.mkOption {
        type = lib.types.str;
        description = "Human-readable tool description for the model.";
      };
      kind = lib.mkOption {
        type = lib.types.str;
        description = "Backend kind (e.g. host.memory, systemd.unit_status, journal.query).";
      };
      enabled = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = "Whether this tool is registered with the agent.";
      };
      params = lib.mkOption {
        type = lib.types.attrs;
        default = { };
        description = "Backend-specific static parameters.";
      };
    };
  };

  settingsToml = pkgs.formats.toml.generate "rsagent-config.toml" {
    llm = {
      base_url = cfg.settings.llm.baseUrl;
      model = cfg.settings.llm.model;
    };

    agent = {
      system_prompt = cfg.settings.agent.systemPrompt;
      max_steps = cfg.settings.agent.maxSteps;
    };

    tools = map (tool: {
      inherit (tool) name description kind enabled;
      params = tool.params;
    }) cfg.settings.tools;
  };

  configFile =
    if cfg.configFile != null then cfg.configFile else settingsToml;

  envFileContent = lib.concatStringsSep "\n" (
    lib.mapAttrsToList (k: v: "${k}=${v}") cfg.environment
  );
in
{
  options.services.rsagent = {
    enable = lib.mkEnableOption "rsagent server inspection CLI";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.callPackage ./package.nix { };
      description = "The rsagent package to install.";
    };

    configFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Optional path to a config.toml file. When set, overrides the declarative
        `settings` option.
      '';
    };

    settings = lib.mkOption {
      type = lib.types.submodule {
        options = {
          llm = lib.mkOption {
            type = lib.types.submodule {
              options = {
                baseUrl = lib.mkOption {
                  type = lib.types.str;
                  default = "https://api.fireworks.ai/inference/v1";
                  description = "Fireworks inference API base URL.";
                };
                model = lib.mkOption {
                  type = lib.types.str;
                  default = "accounts/fireworks/routers/kimi-k2p6-turbo";
                  description = "Fireworks model or router id.";
                };
                apiKey = lib.mkOption {
                  type = lib.types.nullOr lib.types.str;
                  default = null;
                  description = ''
                    Optional API key embedded in config (discouraged).
                    Prefer environmentFiles with FIREWORKS_API_KEY.
                  '';
                };
              };
            };
            default = { };
          };

          agent = lib.mkOption {
            type = lib.types.submodule {
              options = {
                systemPrompt = lib.mkOption {
                  type = lib.types.str;
                  default = "You are a concise server operations assistant. Use tools to inspect the host.";
                  description = "System prompt for the agent loop.";
                };
                maxSteps = lib.mkOption {
                  type = lib.types.int;
                  default = 10;
                  description = "Maximum agent loop steps (tool rounds).";
                };
              };
            };
            default = { };
          };

          tools = lib.mkOption {
            type = lib.types.listOf toolType;
            default = [ ];
            description = "Config-defined tools registered with the agent.";
          };
        };
      };
      default = { };
      description = "Declarative rsagent configuration rendered as /etc/rsagent/config.toml.";
    };

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = { };
      description = "Non-secret variables written to /etc/rsagent/environment.";
    };

    environmentFiles = lib.mkOption {
      type = lib.types.listOf lib.types.str;
      default = [ ];
      description = ''
        Secret env files appended to /etc/rsagent/environment at activation
        (e.g. sops/agenix paths containing FIREWORKS_API_KEY=...).
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    systemd.tmpfiles.rules = [
      "d /etc/rsagent 0750 root root - -"
    ];

    system.activationScripts.rsagent-setup = lib.stringAfter [ "etc" ] ''
      install -m 0644 -D ${configFile} /etc/rsagent/config.toml

      install -m 0640 /dev/null /etc/rsagent/environment
      cat > /etc/rsagent/environment <<'RSAGENT_ENV_EOF'
${envFileContent}
RSAGENT_ENV_EOF
      ${lib.concatStringsSep "\n" (map (file: ''
        if [ -f "${file}" ]; then
          echo "" >> /etc/rsagent/environment
          cat "${file}" >> /etc/rsagent/environment
        fi
      '') cfg.environmentFiles)}
      chmod 0640 /etc/rsagent/environment
    '';
  };
}
