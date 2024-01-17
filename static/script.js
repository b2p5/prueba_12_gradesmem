document.addEventListener('DOMContentLoaded', function() {
    const url = "http://127.0.0.1:8000/get_descen_html/";
    const intervaloEnMilisegundos = 5000; // 5 segundos, ajusta esto según necesites

    setInterval(() => {
        fetch(url)
            .then(response => response.text()) // Asumiendo que la respuesta es texto/HTML
            .then(data => {
                document.getElementById('apiResult').innerHTML = data;
            })
            .catch(error => console.error('Error al obtener datos:', error));
    }, intervaloEnMilisegundos);
});
