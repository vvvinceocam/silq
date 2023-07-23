<?php
use Silq\HttpClient;

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

test('POST request with string body', function () {
    $expectedContent = "Some content\non multiple lines";
    $client = new HttpClient();
    $request = $client->post('http://localhost:8080');
    $response = $request
        ->withBody($expectedContent)
        ->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('POST');
    expect($json['path'])->toBe('/');
    expect($json['headers']['host'])->toBe('localhost:8080');
    expect($json['body'])->toBe($expectedContent);
});

test('POST request with JSON body', function () {
    $expectedContent = [
        "string" => "some value",
        "integer" => 123,
        "float" => 3.14,
        "array" => [ 1, 2, 3 ],
        "null" => NULL,
    ];
    $client = new HttpClient();
    $request = $client->post('http://localhost:8080');
    $response = $request
        ->withJson($expectedContent)
        ->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('POST');
    expect($json['path'])->toBe('/');
    expect($json['headers']['content-type'])->toBe('application/json');
    expect($json['json'])->toBe($expectedContent);
});

test('POST request with FORM body', function () {
    $expectedContent = [
        "string" => "some value",
        "integer" => "123",
        "float" => "3.14",
    ];
    $client = new HttpClient();
    $request = $client->post('http://localhost:8080');
    $response = $request
        ->withForm($expectedContent)
        ->send();

    expect($response->getStatusCode())->toBe(200);

    $body = $response->getText();
    $json = json_decode($body, true);

    expect($json['method'])->toBe('POST');
    expect($json['path'])->toBe('/');
    expect($json['headers']['content-type'])->toBe('application/x-www-form-urlencoded');
    parse_str($json['body'], $parsed);
    expect($parsed)->toBe($expectedContent);
});

test('GET request with basic auth', function () {
    $client = new HttpClient();
    $response = $client->get('https://postman-echo.com/basic-auth')->withBasicAuth('postman', 'password')->send();
    expect($response->getStatusCode())->toBe(200);
    $body = $response->getText();
    $json = json_decode($body, true);
    expect($json['authenticated'])->toBeTrue();
});

test('fetch and parse JSON body', function () {
    $expectedContent = [
        "string" => "some value",
        "integer" => 123,
        "float" => 3.14,
        "array" => [ 1, 2, 3 ],
        "null" => NULL,
    ];
    $client = new HttpClient();
    $request = $client->post('http://localhost:8080');
    $response = $request
        ->withJson($expectedContent)
        ->send();

    expect($response->getStatusCode())->toBe(200);

    $json = $response->getJson();
    expect($json['json'])->toBe($expectedContent);
});

test('consume body frame by frame', function () {
    $expectedContent = [
        "as" => str_repeat('a', 8000),
        "bs" => str_repeat('b', 8000),
        "cs" => str_repeat('c', 8000),
        "ds" => str_repeat('d', 8000),
        "es" => str_repeat('e', 8000),
    ];
    $client = new HttpClient();
    $request = $client->post('http://localhost:8080');
    $response = $request
        ->withJson($expectedContent)
        ->send();

    expect($response->getStatusCode())->toBe(200);

    $content = '';
    foreach ($response->iterFrames() as $frame) {
        $content .= $frame;
    }

    $json = json_decode($content, true);
    expect($json['json'])->toBe($expectedContent);
});

test('get response header', function () {
    $client = new HttpClient();
    $request = $client->get('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $value = $response->getHeaderFirstValue('CONTENT-TYPE');
    expect($value)->toBe("application/json; charset=utf-8");

    $values = $response->getHeaderAllValues('content-type');
    expect($values)->toBe(["application/json; charset=utf-8"]);
});

test('list response headers', function () {
    $client = new HttpClient();
    $request = $client->get('http://localhost:8080');
    $response = $request->send();

    expect($response->getStatusCode())->toBe(200);

    $i = 0;
    foreach ($response->iterHeaders() as $j => [$name, $value]) {
        expect($j)->toBe($i);
        expect($name)->not->toBe('');
        expect($value)->not->toBe('');
        $i++;
    }
    expect($i)->toBeGreaterThan(0);
});
