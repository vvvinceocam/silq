<?php
use Spidroin\HttpClient;

function parseSafeCookies(string $rawCookies): array {
    $pairs = array_map(fn($cookie) => explode('=', $cookie), explode('; ', $rawCookies));
    return array_combine(array_column($pairs, 0), array_map(fn($value) => urldecode($value), array_column($pairs, 1)));
}

test('simple GET request', function () {
    $client = new HttpClient();
    $request = $client->get('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('GET');
    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
});

test('simple POST request', function () {
    $client = new HttpClient();
    $request = $client->post('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('POST');
    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
});

test('simple PUT request', function () {
    $client = new HttpClient();
    $request = $client->put('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('PUT');
    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
});

test('simple PATCH request', function () {
    $client = new HttpClient();
    $request = $client->patch('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('PATCH');
    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
});

test('simple DELETE request', function () {
    $client = new HttpClient();
    $request = $client->delete('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('DELETE');
    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
});

test('GET request with headers', function () {
    $headers = [
       'x-custom-header1' => 'some value',
       'x-custom-header2' => 'some value with ;',
   ];

    $client = new HttpClient();
    $request = $client->get('http://localhost:8080');
    $response = $request
        ->withHeaders($headers)
        ->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['path'])->toBe('/');
    expect($json['headers'])->toMatchArray($headers);
});

test('GET request with cookies', function () {
    $cookies = [
        "baz" => "qux",
        "foo" => "bar",
        "some-key" => "value2 ;  value2",
    ];
    $client = new HttpClient();
    $response = $client->get('http://localhost:8080')->withSafeCookies($cookies)->send();
    expect($response->getStatusCode())->toBe(200);
    $body = $response->getText();
    $json = json_decode($body, true);
    expect(parseSafeCookies($json['headers']['cookie']))->toMatchArray($cookies);
});

test('GET request with both cookies and headers', function () {
    $headers = [
       'x-custom-header1' => 'some value',
       'x-custom-header2' => 'some value with ;',
   ];

    $cookies = [
        "baz" => "qux",
        "foo" => "bar",
    ];

    $client = new HttpClient();
    $response = $client->get('http://localhost:8080')
        ->withHeaders($headers)
        ->withSafeCookies($cookies)
        ->send();
    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect(parseSafeCookies($json['headers']['cookie']))->toMatchArray($cookies);
    expect($json['headers'])->toMatchArray($headers);
});
