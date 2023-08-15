<?php
use Silq\HttpClient;
use Silq\CertificateAuthority;
use Silq\ClientIdentity;

beforeAll(function () {
    global $mtlsData;

    $client = HttpClient::default();
    $mtlsData['ca'] = file_get_contents('tests/data/ca-crt.pem');
    $mtlsData['key'] = file_get_contents('tests/data/client1-key.pem');
    $mtlsData['cert'] = file_get_contents('tests/data/client1-crt.pem');
});

test('mTLS connection with PEM files to domain', function () {
    global $mtlsData;

    $client = HttpClient::builder()
        ->withServerAuthentication(CertificateAuthority::fromPem($mtlsData['ca']))
        ->withClientAuthentication(ClientIdentity::fromPem($mtlsData['cert'], $mtlsData['key']))
        ->build();

    $response = $client->get('https://localhost:8443/')->send();
    expect($response->getStatusCode())->toBe(200);
});

test('mTLS connection with PEM files to IPv4', function () {
    global $mtlsData;

    $client = HttpClient::builder()
        ->withServerAuthentication(CertificateAuthority::fromPem($mtlsData['ca']))
        ->withClientAuthentication(ClientIdentity::fromPem($mtlsData['cert'], $mtlsData['key']))
        ->build();

    $response = $client->get('https://127.0.0.1:8443/')->send();
    expect($response->getStatusCode())->toBe(200);
});

test('mTLS connection with PEM files to IPv6', function () {
    global $mtlsData;

    $client = HttpClient::builder()
        ->withServerAuthentication(CertificateAuthority::fromPem($mtlsData['ca']))
        ->withClientAuthentication(ClientIdentity::fromPem($mtlsData['cert'], $mtlsData['key']))
        ->build();

    $response = $client->get('https://[::1]:8443/')->send();
    expect($response->getStatusCode())->toBe(200);
});

test('mTLS connection with base64 encoded PEM files', function () {
    global $mtlsData;

    $base64CA = base64_encode($mtlsData['ca']);
    $base64key = base64_encode($mtlsData['key']);
    $base64cert = base64_encode($mtlsData['cert']);

    $client = HttpClient::builder()
        ->withServerAuthentication(CertificateAuthority::fromBase64Pem($base64CA))
        ->withClientAuthentication(ClientIdentity::fromBase64Pem($base64cert, $base64key))
        ->build();

    $response = $client->get('https://localhost:8443/')->send();
    expect($response->getStatusCode())->toBe(200);
});

test('raise exception on unknown issuer', function () {
    global $mtlsData;

    $client = HttpClient::builder()
        ->withServerAuthentication(CertificateAuthority::fromPem($mtlsData['ca']))
        ->withClientAuthentication(ClientIdentity::fromPem($mtlsData['cert'], $mtlsData['key']))
        ->build();

    expect(fn() => $client->get('https://google.ch')->send())
        ->toThrow(new Exception('Silq Exception: Connection error: invalid peer certificate: UnknownIssuer'));
});