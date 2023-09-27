const base_url = import.meta.env.VITE_BASE_URL

export const getGoogleUrl = (from: string) => {
  const rootUrl = `https://accounts.google.com/o/oauth2/v2/auth`;

  const options = {
    redirect_uri: `${base_url}/api/sessions/oauth/google`,
    client_id: "664955166234-41kij7p9e1ao2j0th5lid6n1ipk4iikb.apps.googleusercontent.com",
    access_type: "offline",
    response_type: "code",
    prompt: "consent",
    scope: [
      "https://www.googleapis.com/auth/userinfo.profile",
      "https://www.googleapis.com/auth/userinfo.email",
    ].join(" "),
    state: from,
  };

  const qs = new URLSearchParams(options);

  return `${rootUrl}?${qs.toString()}`;
};

