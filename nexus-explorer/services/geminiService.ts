import { GoogleGenAI } from "@google/genai";

const apiKey = process.env.API_KEY || '';
const ai = new GoogleGenAI({ apiKey });

export const analyzeCode = async (code: string, filename: string): Promise<string> => {
  if (!apiKey) {
    return "API Key not configured. Please set the API_KEY environment variable to use the AI analysis feature.";
  }

  try {
    const response = await ai.models.generateContent({
      model: 'gemini-2.5-flash',
      contents: `You are an expert code reviewer. Briefly summarize what this file (${filename}) does in one or two short sentences. Keep it technical but concise for a CLI output.
      
      File Content:
      ${code}
      `,
    });

    return response.text || "No analysis available.";
  } catch (error) {
    console.error("Gemini API Error:", error);
    return "Failed to analyze code. Please try again later.";
  }
};
