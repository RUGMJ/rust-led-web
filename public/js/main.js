const color1 = document.querySelector('#color1');
const color2 = document.querySelector('#color2');
const root = document.querySelector(':root');

setInterval(refresh, 100);

color1.addEventListener('change', change);
color2.addEventListener('change', change);

async function refresh() {
	const res = await fetch('/api/gradient');
	const json = await res.json();

	const { col1, col2 } = json;

	color1.value = `${col1}`;
	color2.value = `${col2}`;

	setProperties(col1, col2);
}

function setProperties(col1, col2) {
	root.style.setProperty(
		'--background',
		`linear-gradient(to right, ${col1}, ${col2})`,
	);
}

async function change(e) {
	e.preventDefault();
	const col1 = color1.value.replace('#', '');
	const col2 = color2.value.replace('#', '');
	console.log(col1, col2);

	await fetch(`/api/gradient/${col1}/${col2}`);
}

function invertColor(hex) {
	if (hex.indexOf('#') === 0) {
		hex = hex.slice(1);
	}

	if (hex.length === 3) {
		hex = hex[0] + hex[0] + hex[1] + hex[1] + hex[2] + hex[2];
	}

	if (hex.length !== 6) {
		throw new Error('Invalid HEX color.');
	}

	var r = parseInt(hex.slice(0, 2), 16),
		g = parseInt(hex.slice(2, 4), 16),
		b = parseInt(hex.slice(4, 6), 16);
	// https://stackoverflow.com/a/3943023/112731
	return r * 0.299 + g * 0.587 + b * 0.114 > 186 ? '#000000' : '#FFFFFF';
}
