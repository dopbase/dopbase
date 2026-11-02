import { apiRequest } from "./http.client";

/**
 * A provider preset. The server reads them from
 * `app/src/modules/sso/presets.toml`, so the list here is never hard-coded.
 */
export interface SsoPreset {
  id: string;
  /** The name in the provider list. */
  label: string;
  /** The default sign-in button label. */
  buttonLabel: string;
  /** An example issuer URL. */
  issuer: string;
  hint: string;
}

/** The preset every provider falls back to. */
export const GENERIC_PRESET = "generic";

export interface SsoProvider {
  id: string;
  /** The ID of an entry in `SsoSettings.presets`. */
  preset: string;
  displayName: string;
  issuer: string;
  clientId: string;
  enabled: boolean;
  /** Password login is refused for everyone except root. */
  requireSso: boolean;
  /** The login page shows only the single sign-on button. */
  hidePasswordForm: boolean;
  /** Set once root completes a test sign-in with the current settings. */
  verifiedAt: string | null;
  linkedAccounts: number;
  /** Admins and members who lose access if single sign-on goes away. */
  passwordlessAccounts: number;
  updatedAt: string;
}

export interface SsoSettings {
  provider: SsoProvider | null;
  /** Register this redirect URI with the identity provider. */
  redirectUri: string;
  /** The server's public URL, which the redirect URI is built from. */
  publicUrl: string;
  /** Single sign-on cannot be set up or used unless this is true. */
  publicUrlSecure: boolean;
  /** The providers to pick from, in display order. */
  presets: SsoPreset[];
}

export interface SaveSsoProviderRequest {
  preset: string;
  displayName: string;
  issuer: string;
  clientId: string;
  /** Leave out to keep the stored secret. */
  clientSecret?: string;
}

const BASE = "/api/v1/sso/provider";

export async function fetchSsoSettings(
  signal?: AbortSignal,
): Promise<SsoSettings> {
  const { data } = await apiRequest<SsoSettings>(BASE, { signal });
  return data;
}

export async function saveSsoProvider(
  request: SaveSsoProviderRequest,
): Promise<SsoSettings> {
  const { data } = await apiRequest<SsoSettings>(BASE, {
    method: "PUT",
    body: request,
  });
  return data;
}

export async function setSsoEnabled(enabled: boolean): Promise<SsoSettings> {
  const { data } = await apiRequest<SsoSettings>(`${BASE}/enabled`, {
    method: "PUT",
    body: { enabled },
  });
  return data;
}

export async function deleteSsoProvider(): Promise<void> {
  await apiRequest(BASE, { method: "DELETE" });
}

export async function setSsoRequired(required: boolean): Promise<SsoSettings> {
  const { data } = await apiRequest<SsoSettings>(`${BASE}/required`, {
    method: "PUT",
    body: { required },
  });
  return data;
}

export async function setSsoPasswordFormHidden(
  hidden: boolean,
): Promise<SsoSettings> {
  const { data } = await apiRequest<SsoSettings>(`${BASE}/password-form`, {
    method: "PUT",
    body: { hidden },
  });
  return data;
}
