/// Classify failures without echoing URLs, codes, secrets, or response bodies.
String malAuthErrorSummary(String raw) {
  final lower = raw.toLowerCase();
  if (lower.contains('keyring') ||
      lower.contains('secret service') ||
      lower.contains('no default store')) {
    return 'Secure keyring storage failed. Unlock the Linux keyring and retry.';
  }
  if (lower.contains('invalid_client') || lower.contains('http 401')) {
    return 'MAL rejected the client. Check the Client ID and, if required by your app, Client Secret.';
  }
  if (lower.contains('invalid_grant')) {
    return 'MAL rejected the authorization. Start a fresh browser login.';
  }
  if (lower.contains('state did not match')) {
    return 'The browser callback could not be verified. Start a fresh login.';
  }
  if (lower.contains('timed out waiting')) {
    return 'The browser callback timed out. Start a fresh login and complete it before retrying.';
  }
  if (lower.contains('invalid_request') || lower.contains('mal_client_id')) {
    return 'Check the Client ID and the redirect URI registered with MAL, then retry.';
  }
  if (lower.contains('denied')) {
    return 'MAL authorization was denied or canceled.';
  }
  final http = RegExp(r'\bhttp\s+(\d{3})\b').firstMatch(lower)?.group(1);
  return http != null
      ? 'MAL authorization failed (HTTP $http). Check connectivity and retry.'
      : 'MAL authorization did not complete. Check client configuration and retry.';
}
