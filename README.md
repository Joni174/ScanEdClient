# ScanEd-Client

Die Client-Anwendung der Diplomarbeit ScanEd.

## Docker

Für das Starten des Clients wurde ein Dockerfile vorbereitet. 
Dieses beinhaltet auch Opendronemap als Abhängigkeit. 

Erstellen des Images:
 
```
docker build -t scaned-client .
```

Erstellen und Ausführen des Containers:

```
docker run -p 8080:8080 scaned-client 
```

Nun ist der Docker-Contaier unter "http://localhost:8080/ erreichbar.

## Verbindung zum ScanEd-Server:

Der ScanEd-Server wird auf dem Raspberry-Pi beim starten ausgeführt. 
Dieser stellt ein Wlan-Netzwerk zur Verfügung. Zu diesem muss sich 
mittels Eingabe der IP Addresse "192.168.1.2" im Webinterface
des Clients verbunden werden. 