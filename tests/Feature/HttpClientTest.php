<?php
 use Spidroin\HttpClient;

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
