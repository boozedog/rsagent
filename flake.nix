{
  description = "Config-defined server agent CLI for NixOS hosts";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default";
  };

  outputs =
    {
      self,
      nixpkgs,
      systems,
    }:
    let
      eachSystem = f: nixpkgs.lib.genAttrs (import systems) f;
    in
    {
      packages = eachSystem (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          default = pkgs.callPackage ./nix/package.nix { };
          rsagent = self.packages.${system}.default;
        }
      );

      nixosModules.default = ./nix/nixos-module.nix;
      nixosModules.rsagent = self.nixosModules.default;

      checks = nixpkgs.lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ] (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          eval = nixpkgs.lib.nixosSystem {
            inherit system;
            modules = [
              self.nixosModules.default
              {
                services.rsagent = {
                  enable = true;
                  settings.tools = [
                    {
                      name = "memory_usage";
                      description = "Memory";
                      kind = "host.memory";
                    }
                  ];
                };
              }
            ];
          };
        in
        {
          nixos-module-eval = pkgs.runCommand "rsagent-nixos-module-eval" { } ''
            grep -q rsagent-setup ${eval.config.system.build.toplevel}/activate
            touch $out
          '';
        }
      );

      formatter = eachSystem (system: nixpkgs.legacyPackages.${system}.nixfmt-rfc-style);
    };
}
