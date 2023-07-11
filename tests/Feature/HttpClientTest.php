<?php
 use Spidroin\HttpClient;

test('simple get', function () {
    $client = new HttpClient();
    $response = $client->get('http://localhost:8080');

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
});
