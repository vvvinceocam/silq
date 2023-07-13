<?php
 use Spidroin\HttpClient;

test('simple GET request', function () {
    $client = new HttpClient();
    $request = $client->get('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

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