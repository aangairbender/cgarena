import { ArenaConfiguration, ConfigurationState } from "@/models";

export interface ConfigurationAdapter {
  fetch(): Promise<ConfigurationState>;
  apply(candidate: ArenaConfiguration): Promise<ConfigurationState>;
}

export const configurationQueryKey = ["configuration"] as const;
