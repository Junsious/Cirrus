async function fetchFiles() {
    const response = await fetch('/list-files');
    const files = await response.json();
    const fileList = document.getElementById('fileList');
    fileList.innerHTML = ''; // Clear the current list

    files.forEach(file => {
        const listItem = document.createElement('li');
        const link = document.createElement('a');
        link.href = `/download/${file}`;
        link.innerText = file;
        link.download = file;

        listItem.appendChild(link);
        fileList.appendChild(listItem);
    });
}

// Load the file list on page load
document.addEventListener('DOMContentLoaded', fetchFiles);

// Refresh the file list after uploading a file
const uploadForm = document.getElementById('uploadForm');
uploadForm.addEventListener('submit', async (e) => {
    e.preventDefault(); // Prevent default form submission
    const formData = new FormData(uploadForm);

    // Send the file upload request
    const response = await fetch('/upload', {
        method: 'POST',
        body: formData,
    });

    if (!response.ok) {
        const errorMessage = await response.text();
        console.error('Upload failed:', errorMessage);
        alert('Upload failed: ' + errorMessage);
    } else {
        // After successful upload, refresh the file list
        fetchFiles();
    }
});
