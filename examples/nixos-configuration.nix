# Example NixOS snippet — import the flake and enable the module.
#
# inputs.rsagent.url = "path:/path/to/rsagent";
# inputs.rsagent.url = "git+ssh://git@your-forge/rsagent.git";
#
# modules = [ inputs.rsagent.nixosModules.default ];

{
  services.rsagent = {
    enable = true;

    settings = {
      llm = {
        baseUrl = "https://api.fireworks.ai/inference/v1";
        model = "accounts/fireworks/routers/kimi-k2p6-turbo";
      };

      tools = [
        {
          name = "memory_usage";
          description = "Current host memory usage";
          kind = "host.memory";
        }
        {
          name = "nginx_status";
          description = "nginx.service status";
          kind = "systemd.unit_status";
          params = {
            allowed_units = [ "nginx.service" ];
          };
        }
        {
          name = "nginx_errors";
          description = "Recent nginx journal errors";
          kind = "journal.query";
          params = {
            unit = "nginx";
            since = "-1h";
            priority = "3";
            max_lines = 100;
          };
        }
      ];
    };

    # Append secrets at activation (agenix, sops-nix, etc.)
    environmentFiles = [
      # config.age.secrets.rsagent-env.path
    ];
  };
}
