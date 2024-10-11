document.addEventListener("DOMContentLoaded", () => {
    const uploadForm = document.getElementById('uploadForm');

    // Function to update the list of files
    const fetchFiles = async () => {
        try {
            const response = await fetch('/files');
            const files = await response.json();
            const fileList = document.getElementById('fileList');
            fileList.innerHTML = ''; // Clear the list before updating

            files.forEach(file => {
                const li = document.createElement('li');
                li.textContent = file;
                fileList.appendChild(li);
            });
        } catch (error) {
            console.error('Error fetching files:', error);
        }
    };

    // Submitting the form with multiple files
    uploadForm.addEventListener('submit', async (e) => {
        e.preventDefault();
        const formData = new FormData();
        const files = document.getElementById('fileInput').files;

        for (let i = 0; i < files.length; i++) {
            formData.append('files[]', files[i]); // Add all files to FormData
        }

        try {
            const response = await fetch('/upload', {
                method: 'POST',
                body: formData,
            });

            if (response.ok) {
                alert('Files uploaded successfully');
                fetchFiles(); // Update the file list after upload
                document.getElementById('fileInput').value = ''; // Clear the file input field
            } else {
                alert('Failed to upload files');
            }
        } catch (error) {
            alert('Error: ' + error.message);
        }
    });

    // Initialize the file list on page load
    fetchFiles();
});
